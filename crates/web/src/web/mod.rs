pub mod routes {
    pub mod apis;
    pub mod assets;
    pub mod pages;
}

pub mod modules {
    pub mod settings;
}

pub mod functions {
    pub mod general;
    pub mod search_db;
    pub mod search_endpoint;

    pub mod ranking {
        pub mod hand;
        pub mod rrf;
    }

    pub mod additional {
        pub mod discover;
        pub mod spell_check;
    }

    pub mod search_api {
        pub mod all;
        pub mod img;
        pub mod news;
        pub mod video;
        pub mod yadore;
    }
}
