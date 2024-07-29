pub mod service;
mod types;
use consensus_core::BlockAPI;
use fastcrypto::hash::Hash;
use narwhal_types::{BatchAPI, CertificateAPI, HeaderAPI};
pub use service::consensus_api_server::*;
pub use types::{
    Block, ConsensusOutput, Empty, ExternalTransaction, ReputationScore, RequestEcho, ResponseEcho,
    ValidatorInfo, ValidatorState,
};

use crate::consensus_types::{consensus_output_api::ConsensusOutputAPI, AuthorityIndex};

impl From<narwhal_types::ConsensusOutput> for ConsensusOutput {
    fn from(value: narwhal_types::ConsensusOutput) -> Self {
        assert!(value.sub_dag.certificates.len() == value.batches.len());
        let commit_digest = value.digest().into_inner().to_vec();
        let reputation_scores = value
            .reputation_score_sorted_desc()
            .map(|scores| {
                scores
                    .iter()
                    .map(|(index, score)| ReputationScore {
                        authority_index: index.clone(),
                        score: score.clone(),
                    })
                    .collect::<Vec<ReputationScore>>()
            })
            .unwrap_or_default();
        let mut consensus_output = ConsensusOutput {
            leader_round: value.leader_round(),
            leader_author_index: value.leader_author_index(),
            commit_timestamp: value.commit_timestamp_ms(),
            commit_sub_dag_index: value.commit_sub_dag_index(),
            commit_digest,
            reputation_scores,
            blocks: vec![],
        };
        let narwhal_types::ConsensusOutput { sub_dag, batches } = value;
        consensus_output.blocks = sub_dag
            .certificates
            .iter()
            .zip(batches)
            .map(|(cert, batches)| {
                assert_eq!(cert.header().payload().len(), batches.len());
                let transactions: Vec<Vec<u8>> = batches
                    .iter()
                    .flat_map(|batch| {
                        let digest = batch.digest();
                        assert!(cert.header().payload().contains_key(&digest));
                        //Transactions come to the consensus in format ChainTransaction {chain_id, transaction: Vec<u8>}
                        //Extract transaction part from batch
                        batch.transactions().clone()
                    })
                    .collect();
                Block {
                    authority_index: cert.origin().0 as AuthorityIndex,
                    transactions,
                }
            })
            .collect();
        consensus_output
    }
}

//Recovery ConsensusOuput from consensus_core::CommittedSubDag
impl From<consensus_core::CommittedSubDag> for ConsensusOutput {
    fn from(value: consensus_core::CommittedSubDag) -> Self {
        let commit_digest = value.commit_ref.digest.into_inner().to_vec();
        let reputation_scores = value
            .reputation_score_sorted_desc()
            .map(|scores| {
                scores
                    .iter()
                    .map(|(index, score)| ReputationScore {
                        authority_index: index.clone(),
                        score: score.clone(),
                    })
                    .collect::<Vec<ReputationScore>>()
            })
            .unwrap_or_default();
        let mut consensus_output = ConsensusOutput {
            leader_round: value.leader_round(),
            leader_author_index: value.leader_author_index(),
            commit_timestamp: value.commit_timestamp_ms(),
            commit_sub_dag_index: value.commit_sub_dag_index(),
            commit_digest,
            reputation_scores,
            blocks: vec![],
        };
        let consensus_core::CommittedSubDag { blocks, .. } = value;
        consensus_output.blocks = blocks
            .into_iter()
            .map(|block| {
                let authority_index = block.author().value() as AuthorityIndex;
                let transactions = block
                    .transactions()
                    .iter()
                    .map(|tx| tx.data().to_vec())
                    .collect();
                Block {
                    authority_index,
                    transactions,
                }
            })
            .collect();
        consensus_output
    }
}
