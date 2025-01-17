use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use futures::future::BoxFuture;
use jmt::{
    storage::{Node, NodeBatch, NodeKey, TreeReader, TreeWriter},
    WriteOverlay,
};
use rocksdb::DB;
use tokio::sync::Mutex;
use tracing::{instrument, Span};

mod overlay_ext;

pub use overlay_ext::OverlayExt;

pub type Overlay = Arc<Mutex<WriteOverlay<Storage>>>;

#[derive(Clone, Debug)]
pub struct Storage(Arc<DB>);

impl Storage {
    pub async fn load(path: PathBuf) -> Result<Self> {
        let span = Span::current();
        tokio::task::Builder::new()
            .name("open_rocksdb")
            .spawn_blocking(move || {
                span.in_scope(|| {
                    tracing::info!(?path, "opening rocksdb");
                    Ok(Self(Arc::new(DB::open_default(path)?)))
                })
            })
            .await
            .unwrap()
    }

    /// Returns the latest version (block height) of the tree recorded by the
    /// `Storage`, or `None` if the tree is empty.
    pub async fn latest_version(&self) -> Result<Option<jmt::Version>> {
        Ok(self
            .get_rightmost_leaf()
            .await?
            .map(|(node_key, _)| node_key.version()))
    }

    /// Returns a new [`Overlay`] on top of the latest version of the tree.
    pub async fn overlay(&self) -> Result<Overlay> {
        // If the tree is empty, use PRE_GENESIS_VERSION as the version,
        // so that the first commit will be at version 0.
        let version = self
            .latest_version()
            .await?
            .unwrap_or(WriteOverlay::<Storage>::PRE_GENESIS_VERSION);

        tracing::debug!("creating overlay for version {}", version);
        Ok(Arc::new(Mutex::new(WriteOverlay::new(
            self.clone(),
            version,
        ))))
    }

    /// Like [`Self::overlay`], but bundles in a [`tonic`] error conversion.
    ///
    /// This is useful for implementing gRPC services that query the storage:
    /// each gRPC request can create an ephemeral [`Overlay`] pinning the current
    /// version at the time the request was received, and then query it using
    /// component `View`s to handle the request.
    pub async fn overlay_tonic(&self) -> std::result::Result<Overlay, tonic::Status> {
        self.overlay()
            .await
            .map_err(|e| tonic::Status::internal(e.to_string()))
    }
}

impl TreeWriter for Storage {
    /// Writes a node batch into storage.
    //TODO: Change JMT traits to remove/simplify lifetimes & accept owned NodeBatch
    #[instrument(skip(self, node_batch))]
    fn write_node_batch<'future, 'a: 'future, 'n: 'future>(
        &'a mut self,
        node_batch: &'n NodeBatch,
    ) -> BoxFuture<'future, Result<()>> {
        let db = self.0.clone();
        let node_batch = node_batch.clone();

        // The writes have to happen on a separate spawn_blocking task, but we
        // want tracing events to occur in the context of the current span, so
        // propagate it explicitly:
        let span = Span::current();

        Box::pin(async {
            tokio::task::Builder::new()
                .name("Storage::write_node_batch")
                .spawn_blocking(move || {
                    span.in_scope(|| {
                        for (node_key, node) in node_batch.clone() {
                            let key_bytes = &node_key.encode()?;
                            let value_bytes = &node.encode()?;
                            tracing::trace!(?key_bytes, value_bytes = ?hex::encode(&value_bytes));
                            db.put(key_bytes, value_bytes)?;
                        }

                        Ok(())
                    })
                })
                .await
                .unwrap()
        })
    }
}

/// A reader interface for rocksdb. NOTE: it is up to the caller to ensure consistency between the
/// rocksdb::DB handle and any write batches that may be applied through the writer interface.
impl TreeReader for Storage {
    /// Gets node given a node key. Returns `None` if the node does not exist.
    #[instrument(skip(self))]
    fn get_node_option<'future, 'a: 'future, 'n: 'future>(
        &'a self,
        node_key: &'n NodeKey,
    ) -> BoxFuture<'future, Result<Option<Node>>> {
        let db = self.0.clone();
        let node_key = node_key.clone();

        let span = Span::current();

        Box::pin(async {
            tokio::task::Builder::new()
                .name("Storage::get_node_option")
                .spawn_blocking(move || {
                    span.in_scope(|| {
                        let value = db
                            .get_pinned(&node_key.encode()?)?
                            .map(|db_slice| Node::decode(&db_slice))
                            .transpose()?;

                        tracing::trace!(?node_key, ?value);
                        Ok(value)
                    })
                })
                .await
                .unwrap()
        })
    }

    fn get_rightmost_leaf<'future, 'a: 'future>(
        &'a self,
    ) -> BoxFuture<'future, Result<Option<(NodeKey, jmt::storage::LeafNode)>>> {
        let span = Span::current();
        let db = self.0.clone();

        Box::pin(async {
            tokio::task::Builder::new()
                .name("Storage::get_rightmost_leaf")
                .spawn_blocking(move || {
                    span.in_scope(|| {
                        let mut iter = db.raw_iterator();
                        let mut ret = None;
                        iter.seek_to_last();

                        if iter.valid() {
                            let node_key = NodeKey::decode(iter.key().unwrap())?;
                            let node = Node::decode(iter.value().unwrap())?;

                            if let Node::Leaf(leaf_node) = node {
                                ret = Some((node_key, leaf_node));
                            }
                        } else {
                            // There are no keys in the database
                        }
                        Ok(ret)
                    })
                })
                .await
                .unwrap()
        })
    }
}
