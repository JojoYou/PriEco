/*
  Import own libraries
*/
use crate::download;
use prieco_core::MSMARCO_DOCUMENT_DIR;

/*
  Constants
*/
const FILES: &[&str] = &[
    // Corpus: docid \t url \t title \t body
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-docs.tsv.gz",
    // Dev: sparse qrels, for MRR@10
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-docdev-queries.tsv.gz",
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-docdev-qrels.tsv.gz",
    // TREC DL: graded 0-3 qrels, for nDCG@10
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-test2019-queries.tsv.gz",
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-test2020-queries.tsv.gz",
    "https://trec.nist.gov/data/deep/2019qrels-docs.txt",
    "https://trec.nist.gov/data/deep/2020qrels-docs.txt",
    // Corpus + queries + dev qrels (~1 GB tar.gz)
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/collectionandqueries.tar.gz",
    // TREC DL: same queries as document, passage qrels
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-test2019-queries.tsv.gz",
    "https://msmarco.z22.web.core.windows.net/msmarcoranking/msmarco-test2020-queries.tsv.gz",
    "https://trec.nist.gov/data/deep/2019qrels-pass.txt",
    "https://trec.nist.gov/data/deep/2020qrels-pass.txt",
];

pub async fn run() {
    // Get dataset
    if !download::run(MSMARCO_DOCUMENT_DIR, FILES).await {
        return;
    }
}
