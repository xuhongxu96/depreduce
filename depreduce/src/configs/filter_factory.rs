use crate::filters::{Filterable, FunctionCallFilter};

pub(super) fn filter_factory(name: &str) -> Box<dyn Filterable> {
    Box::new(match name {
        "no_select" => FunctionCallFilter {
            func_id: "select".to_string(),
        },
        _ => panic!("Unknown filter: {}", name),
    })
}
