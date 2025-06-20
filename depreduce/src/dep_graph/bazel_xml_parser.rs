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
    pub value: Option<String>,
}

// #[derive(Debug, Deserialize)]
// pub enum ListItem {
//     #[serde(rename = "string")]
//     String(StringProp),

//     #[serde(rename = "label")]
//     Label(StringProp),

//     #[serde(rename = "output")]
//     Output(StringProp),
// }

#[derive(Debug, Deserialize)]
pub struct ListProp {
    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "$value")]
    pub items: Option<Vec<VariantProp>>,
}

#[derive(Debug, Deserialize)]
pub struct Pair {
    #[serde(rename = "$value")]
    pub items: Option<Vec<VariantProp>>,
}

#[derive(Debug, Deserialize)]
pub struct DictProp {
    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "pair")]
    pub pairs: Vec<Pair>,
}

#[derive(Debug, Deserialize)]
pub struct RuleIO {
    #[serde(rename = "@name")]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub enum VariantProp {
    #[serde(rename = "string")]
    String(StringProp),

    #[serde(rename = "label")]
    Label(StringProp),

    #[serde(rename = "output")]
    Output(StringProp),

    #[serde(rename = "list")]
    List(ListProp),

    #[serde(rename = "dict")]
    Dict(DictProp),

    #[serde(rename = "boolean")]
    Boolean(StringProp),

    #[serde(rename = "rule-input")]
    RuleInput(RuleIO),

    #[serde(rename = "rule-output")]
    RuleOutput(RuleIO),
}

#[derive(Debug, Deserialize)]
pub struct Rule {
    #[serde(rename = "@class")]
    pub class: String,

    #[serde(rename = "@location")]
    pub location: String,

    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "$value")]
    pub props: Option<Vec<VariantProp>>,
}

#[derive(Debug, Deserialize)]
pub struct GeneratedFile {
    #[serde(rename = "@generating-rule")]
    pub generating_rule: String,

    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "@location")]
    pub location: String,
}

#[derive(Debug, Deserialize)]
pub struct PackageGroup {
    #[serde(rename = "@name")]
    pub name: String,

    #[serde(rename = "@location")]
    pub location: String,

    #[serde(rename = "list")]
    pub list_props: Option<Vec<ListProp>>,
}

#[derive(Debug, Deserialize)]
pub enum SkyValue {
    #[serde(rename = "source-file")]
    SourceFile(SourceFile),

    #[serde(rename = "rule")]
    Rule(Rule),

    #[serde(rename = "generated-file")]
    GeneratedFile(GeneratedFile),

    #[serde(rename = "package-group")]
    PackageGroup(PackageGroup),
}

#[derive(Debug, Deserialize)]
pub struct Query {
    #[serde(rename = "@version")]
    pub version: i32,

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

    #[test]
    fn test_parse_source_file_multi_lang() {
        run_parse_test(
            "multi-lang-deps.xml",
            "dep_graph/bazel_xml_parser/multi-lang.out",
        );
    }

    #[test]
    fn test_parse_source_file_multi_platform() {
        run_parse_test(
            "multi-platform-deps.xml",
            "dep_graph/bazel_xml_parser/multi-platform.out",
        );
    }
}
