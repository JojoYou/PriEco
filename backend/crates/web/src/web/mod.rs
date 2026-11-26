//! # Web
//!
//! Everything served by the PriEco website.
//!
//! * [routes] - Rocket endpoints.
//! * [modules] - Settings shared by pages.
//! * [functions] - Search, ranking, widgets and remote APIs.

/// Rocket endpoints.
pub mod routes {
    pub mod apis;
    pub mod assets;
    pub mod mcp;
    pub mod pages;
}

/// Settings shared by pages.
pub mod modules {
    pub mod settings;
}

/// Search pipeline and everything around it.
pub mod functions {
    pub mod benchmark;
    pub mod decentralized;
    pub mod general;
    pub mod search_db;
    pub mod search_endpoint;

    /// Ranking and query understanding.
    pub mod ranking {
        pub mod hand;
        pub mod rrf;

        /// Query understanding: intent, entities, synonyms.
        pub mod meaning {
            pub mod call;
            pub mod entities;
            pub mod intent;
            pub mod optimize;
        }

        pub mod goggles;
    }

    /// Extra search features.
    pub mod additional {
        pub mod discover;
        pub mod ip_widget;
        pub mod spell_check;
    }

    /// Results from other services.
    pub mod remote_api {
        pub mod currency;
        pub mod img;
        pub mod news;
        pub mod video;
        pub mod yadore;
    }

    /// Search API.
    pub mod api {
        pub mod manage;
    }
}
