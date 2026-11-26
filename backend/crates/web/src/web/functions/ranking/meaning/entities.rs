//! # Entities
//!
//! Finds entities in query and clarifies intent with them.
//!
//! * **Place:** Unknown or Navigational becomes Local, when there is no name or business.
//! * **Business:** Unknown becomes Navigational.
//! * **Person name:** Unknown or Navigational becomes Informational.
//!
//! ## Metadata
//!
//! * **Author:** Roman Láncoš (<support@prieco.net>)
//! * **License:** AGPL-3.0
//! * Date Created: 2026-07-08
//! * Last Modified: 2026-11-25
//!
//! ## Planned Improvements
//!
//! - [ ] None

use prieco_core::{EntityType, QU_PIPELINE, QueryIntent, TaggedEntity};

/// Finds entities and coordinates.
///
/// Place in user's country wins, otherwise the first one.
///
/// # Arguments
///
/// * `query` - Query.
/// * `final_intent` - Intent from keywords.
/// * `loc` - User location.
///
/// # Returns
///
/// (clarified intent, (lat, lon) of last place, found entities).
pub fn scan_entities(
    query: &mut str,
    mut final_intent: QueryIntent,
    loc: &str,
) -> (QueryIntent, Option<(f32, f32)>, Vec<TaggedEntity>) {
    let tags = QU_PIPELINE.get_tags(query);
    let mut spatial_coords = None;
    let contains_name = tags
        .iter()
        .any(|t| matches!(t.entity_type, EntityType::PersonName | EntityType::Business));

    for tag in tags.iter() {
        match tag.entity_type {
            EntityType::Place => {
                // Extract coordinates
                if let Some(coords) = QU_PIPELINE
                    .archived_places
                    .places
                    .get(tag.matched_text.as_str())
                {
                    // Find place in user location
                    let local_match = coords.iter().find(|c| c.country.eq_ignore_ascii_case(loc));
                    if let Some(local_coord) = local_match {
                        spatial_coords =
                            Some((local_coord.lat.to_native(), local_coord.lon.to_native()));
                    } else if !coords.is_empty() {
                        spatial_coords =
                            Some((coords[0].lat.to_native(), coords[0].lon.to_native()));
                    }
                }

                // Upgrade Unknown or Navigational to Local
                if !contains_name
                    && (final_intent == QueryIntent::Unknown
                        || final_intent == QueryIntent::Navigational)
                {
                    final_intent = QueryIntent::Local;
                }
            }
            EntityType::Business => {
                if final_intent == QueryIntent::Unknown {
                    final_intent = QueryIntent::Navigational;
                }
            }
            EntityType::PersonName => {
                // Upgrade Unknown or Navigational to Informational
                if final_intent == QueryIntent::Unknown || final_intent == QueryIntent::Navigational
                {
                    final_intent = QueryIntent::Informational;
                }
            }
        }
    }

    (final_intent, spatial_coords, tags)
}
