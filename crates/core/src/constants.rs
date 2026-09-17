// Insterter
pub const ID_SIZE: usize = 8;
pub const VECTOR_DIM: usize = 384;
pub const RECORD_SIZE: usize = ID_SIZE + (VECTOR_DIM * 4);

// Pagerank
pub const ID_MAP_FILE: &str = "pagerank/id_map.bin";
pub const FINAL_SCORES: &str = "pagerank/pageranks.bin";

// Blob storage
pub const SEARCHED_K1: u64 = 0xC70F6907A1C9566B;
pub const SEARCHED_K2: u64 = 0x85E4C66FC71D33EF;
