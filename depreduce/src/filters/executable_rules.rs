use std::collections::HashSet;

use regex::Regex;

pub struct ExecutableRules {
    pub regexes: Vec<Regex>,
    pub names: HashSet<String>,
}

impl ExecutableRules {
    pub fn parse(rules: &[String]) -> Self {
        let mut regexes = Vec::new();
        let mut names = HashSet::new();

        for rule in rules {
            if rule.starts_with("regex:") {
                if let Ok(regex) = Regex::new(&rule["regex:".len()..]) {
                    regexes.push(regex);
                }
            } else {
                names.insert(rule.clone());
            }
        }

        ExecutableRules { regexes, names }
    }

    pub fn is_match(&self, name: &str) -> bool {
        self.is_match_names(name) || self.is_match_regexes(name)
    }

    pub fn is_match_names(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub fn is_match_regexes(&self, name: &str) -> bool {
        self.regexes.iter().any(|rule| rule.is_match(name))
    }
}
