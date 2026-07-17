use prieco_core::{EntityType, QU_PIPELINE, QueryIntent, TaggedEntity};

pub fn scan_entities(
    query: &mut String,
    mut final_intent: QueryIntent,
    loc: &str,
) -> (QueryIntent, Option<(f32, f32)>, Vec<TaggedEntity>) {
    let tags = QU_PIPELINE.get_tags(query);
    let mut spatial_coords = None;

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
                        spatial_coords = Some((local_coord.lat, local_coord.lon));
                    } else if !coords.is_empty() {
                        spatial_coords = Some((coords[0].lat, coords[0].lon)); // Fallback to idx 0
                    }
                }

                // Upgrade Unknown or Navigational to Local
                if final_intent == QueryIntent::Unknown || final_intent == QueryIntent::Navigational
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
