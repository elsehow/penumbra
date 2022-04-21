use std::{collections::BTreeMap, time::SystemTime};

use penumbra_chain::params::ChainParams;
use penumbra_crypto::{asset, merkle::NoteCommitmentTree, note, Note, Nullifier};
use sqlx::{Pool, Sqlite};

use crate::Wallet;

pub struct Storage {
    pub(super) pool: Pool<Sqlite>,
}

impl Storage {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn migrate(self: &Storage) -> anyhow::Result<()> {
        sqlx::migrate!().run(&self.pool).await.map_err(Into::into)
    }

    pub async fn insert_table(self: &Storage) -> anyhow::Result<i64> {
        let mut conn = self.pool.acquire().await?;

        // Insert the task, then obtain the ID of this row
        let id = sqlx::query!(
            r#"
            INSERT INTO penumbra ( value )
            VALUES ( ?1 )
            "#,
            "Hello, world"
        )
        .execute(&mut conn)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    pub async fn read_table(self: &Storage) -> anyhow::Result<String> {
        let recs = sqlx::query!(
            r#"
            SELECT id, value
            FROM penumbra
            ORDER BY id
            LIMIT 1
        "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(recs[0].value.clone())
    }
    //TODO: these should correspond to sqlite tables representing what was previously held in ClientState
    /// The last block height we've scanned to, if any.
    pub async fn last_block_height(&self) -> Option<u64> {
        todo!()
    }
    /// Note commitment tree.
    pub async fn note_commitment_tree(&self) -> NoteCommitmentTree {
        todo!()
    }
    /// Our nullifiers and the notes they correspond to.
    pub async fn nullifier_map(&self) -> BTreeMap<Nullifier, note::Commitment> {
        todo!()
    }
    /// Notes that we have received.
    pub async fn unspent_set(&self) -> BTreeMap<note::Commitment, Note> {
        todo!()
    }
    /// Notes that we have spent but which have not yet been confirmed on-chain.
    pub async fn submitted_spend_set(&self) -> BTreeMap<note::Commitment, (SystemTime, Note)> {
        todo!()
    }
    /// Notes that we anticipate receiving on-chain as change but which have not yet been confirmed.
    pub async fn submitted_change_set(&self) -> BTreeMap<note::Commitment, (SystemTime, Note)> {
        todo!()
    }
    /// Notes that we have spent.
    pub async fn spent_set(&self) -> BTreeMap<note::Commitment, Note> {
        todo!()
    }
    /// Map of note commitment to full transaction data for transactions we have visibility into.
    pub async fn transactions(&self) -> BTreeMap<note::Commitment, Option<Vec<u8>>> {
        todo!()
    }
    /// Map of asset IDs to (raw) asset denominations.
    pub async fn asset_cache(&self) -> asset::Cache {
        todo!()
    }
    /// Key material.
    pub async fn wallet(&self) -> Wallet {
        todo!()
    }
    /// Global chain parameters. May not have been fetched yet.
    pub async fn chain_params(&self) -> Option<ChainParams> {
        todo!()
    }

    pub async fn chain_id(&self) -> Option<String> {
        self.chain_params().await.map(|p| p.chain_id)
    }
}
