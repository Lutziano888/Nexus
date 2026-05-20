// layout_engine als Library-Crate verfügbar machen.
// Andere Crates (z.B. renderer) können jetzt:
//   use layout_engine::dom::Node;
//   use layout_engine::layout::build_layout_tree;

pub mod dom;
pub mod cssom;
pub mod style;
pub mod layout;
pub mod layout_taffy;
pub mod text_measure;  // ← NEU: Text-Messung abstrahieren