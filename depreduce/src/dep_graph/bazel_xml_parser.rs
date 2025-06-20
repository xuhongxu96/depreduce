use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct VisibilityLabel {
    #[serde(rename = "@name")]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct SourceFile {
    #[serde(rename = "@location")]
    pub location: String,

    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "visibility-label")]
    pub visibility: Option<VisibilityLabel>,
}

#[derive(Debug, Deserialize)]
pub struct StringProp {
    #[serde(rename = "@name")]
    pub name: Option<String>,

    #[serde(rename = "@value")]
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct ListProp {
    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "label")]
    pub labels: Option<Vec<StringProp>>,

    #[serde(rename = "string")]
    pub strings: Option<Vec<StringProp>>,
}

#[derive(Debug, Deserialize)]
pub struct RuleIO {
    #[serde(rename = "@name")]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    #[serde(rename = "@class")]
    pub class: String,

    #[serde(rename = "@location")]
    pub location: String,

    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "string")]
    pub string_props: Option<Vec<StringProp>>,

    #[serde(rename = "list")]
    pub list_props: Option<Vec<ListProp>>,

    #[serde(rename = "label")]
    pub labels: Option<Vec<StringProp>>,

    #[serde(rename = "rule-input")]
    pub inputs: Option<Vec<RuleIO>>,

    #[serde(rename = "rule-output")]
    pub outputs: Option<Vec<RuleIO>>,
}

#[derive(Debug, Deserialize)]
pub enum SkyValue {
    #[serde(rename = "source-file")]
    SourceFile(SourceFile),

    #[serde(rename = "rule")]
    Rule(Rule),
}

#[derive(Debug, Deserialize)]
pub struct Query {
    #[serde(rename = "$value")]
    pub values: Vec<SkyValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::de::from_str;
    use utils::*;

    fn run_parse_test(input_path: &str, output_path: &str) {
        let xml = read_test_data!(input_path);

        let value: Query = from_str(&xml).unwrap();
        let res = format!("{:#?}", value);

        assert_eq!(res, read_or_create_test_data!(output_path, res));
    }

    #[test]
    fn test_parse_source_file() {
        run_parse_test("cxx-deps.xml", "dep_graph/bazel_xml_parser/cxx.out");
    }

    #[test]
    fn test_parse_source_file_java() {
        run_parse_test("java-deps.xml", "dep_graph/bazel_xml_parser/java.out");
    }

    #[test]
    fn test_parse_source_file_kt() {
        run_parse_test("kt-deps.xml", "dep_graph/bazel_xml_parser/kt.out");
    }
}
