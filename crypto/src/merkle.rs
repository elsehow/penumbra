use std::convert::{TryFrom, TryInto};

use ark_ff::{PrimeField, Zero};
use decaf377::{FieldExt, Fq};
use incrementalmerkletree;
pub use incrementalmerkletree::{
    bridgetree::{self, AuthFragment, BridgeTree},
    Altitude, Frontier, Hashable, Position, Recording, Tree,
};
use once_cell::sync::Lazy;
use penumbra_proto::{crypto as pb, Protobuf};
use serde::{Deserialize, Serialize};

use crate::note;

pub const DEPTH: usize = 32;
pub type NoteCommitmentTree = BridgeTree<note::Commitment, { DEPTH as u8 }>;

/// The domain separator used to hash items into the Merkle tree.
pub static MERKLE_DOMAIN_SEP: Lazy<Fq> = Lazy::new(|| {
    Fq::from_le_bytes_mod_order(blake2b_simd::blake2b(b"penumbra.merkle.tree").as_bytes())
});

// Return value from `Tree::authentication_path(value: &note::Commitment)`
pub type Path = (Position, Vec<note::Commitment>);

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "pb::MerkleRoot", into = "pb::MerkleRoot")]
pub struct Root(pub Fq);

impl Protobuf<pb::MerkleRoot> for Root {}

impl std::fmt::Display for Root {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex::encode(&self.0.to_bytes()))
    }
}

impl std::fmt::Debug for Root {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("merkle::Root")
            .field(&hex::encode(&self.0.to_bytes()))
            .finish()
    }
}

impl TryFrom<pb::MerkleRoot> for Root {
    type Error = anyhow::Error;

    fn try_from(root: pb::MerkleRoot) -> Result<Root, Self::Error> {
        let bytes: [u8; 32] = (&root.inner[..]).try_into()?;

        let inner = Fq::from_bytes(bytes)?;

        Ok(Root(inner))
    }
}

impl From<Root> for pb::MerkleRoot {
    fn from(root: Root) -> Self {
        Self {
            inner: root.to_bytes().to_vec(),
        }
    }
}

impl TryFrom<&[u8]> for Root {
    type Error = anyhow::Error;

    fn try_from(slice: &[u8]) -> Result<Root, Self::Error> {
        let bytes: [u8; 32] = slice[..].try_into()?;

        let inner = Fq::from_bytes(bytes)?;

        Ok(Root(inner))
    }
}

impl Root {
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }
}

pub trait TreeExt {
    fn root2(&self) -> Root;
}

impl<T> TreeExt for T
where
    T: Tree<note::Commitment>,
{
    fn root2(&self) -> Root {
        Root(self.root().0)
    }
}

impl Hashable for note::Commitment {
    fn empty_leaf() -> Self {
        note::Commitment(Fq::zero())
    }

    fn combine(level: Altitude, a: &Self, b: &Self) -> Self {
        // extend to build domain sep
        let level_fq: Fq = u8::from(level).into();
        let level_domain_sep: Fq = *MERKLE_DOMAIN_SEP + level_fq;
        note::Commitment(poseidon377::hash_2(&level_domain_sep, (a.0, b.0)))
    }
}
