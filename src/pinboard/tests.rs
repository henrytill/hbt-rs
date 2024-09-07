use super::*;

use std::collections::HashSet;

const TEST_XML_EMPTY: &str = "";

#[test]
fn test_empty() {
    let posts: Vec<Post> = Post::from_xml(TEST_XML_EMPTY).unwrap();
    assert!(posts.is_empty());
}

const TEST_XML_SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<posts user="user">
<post href="https://github.com/janestreet/magic-trace" time="2022-04-23T00:29:36Z" description="janestreet/magic-trace: magic-trace collects and displays high-resolution traces of what a process is doing" extended="tragically intel-only" tag="performance profiling tools" hash="54dab27be2409c987bb17fc06e47a729"  shared="yes"  />
<post href="https://www.intel.com/content/www/us/en/developer/tools/oneapi/vtune-profiler.html#gs.x8oazh" time="2022-04-13T13:12:10Z" description="Fix Performance Bottlenecks with Intel® VTune™ Profiler" extended="" tag="performance profiling tools" hash="2ab1611711c8bb5ed9273b8f4b612fca"  shared="no"  toread="yes" />
<post href="http://kcachegrind.sourceforge.net/html/Home.html" time="2022-04-13T08:01:32Z" description="" extended="" tag="performance profiling tools" hash="0850c315d075a67430db267214b18d13"  shared="no"  />
</posts>
"#;

#[test]
fn test_xml_sample() {
    let expected_tags =
        [String::from("performance"), String::from("profiling"), String::from("tools")];
    let magic_trace = Post::new(
        String::from("https://github.com/janestreet/magic-trace"),
        String::from("2022-04-23T00:29:36Z"),
        Some(String::from("janestreet/magic-trace: magic-trace collects and displays high-resolution traces of what a process is doing")),
        Some(String::from("tragically intel-only")),
        Vec::from(expected_tags.clone()),
        Some(String::from("54dab27be2409c987bb17fc06e47a729")),
        true,
        false,
    );
    let intel = Post::new(
        String::from("https://www.intel.com/content/www/us/en/developer/tools/oneapi/vtune-profiler.html#gs.x8oazh"),
        String::from("2022-04-13T13:12:10Z"),
        Some(String::from("Fix Performance Bottlenecks with Intel® VTune™ Profiler")),
        None,
        Vec::from(expected_tags.clone()),
        Some(String::from("2ab1611711c8bb5ed9273b8f4b612fca")),
        false,
        true,
    );
    let kcachegrind = Post::new(
        String::from("http://kcachegrind.sourceforge.net/html/Home.html"),
        String::from("2022-04-13T08:01:32Z"),
        None,
        None,
        Vec::from(expected_tags.clone()),
        Some(String::from("0850c315d075a67430db267214b18d13")),
        false,
        false,
    );
    let expected = vec![magic_trace, intel, kcachegrind];
    let actual = Post::from_xml(TEST_XML_SAMPLE).unwrap();
    assert_eq!(expected, actual);
    let expected_tags = Tags::new(HashSet::from(expected_tags));
    let actual_tags = Post::tags(&actual);
    assert_eq!(expected_tags, actual_tags)
}

const TEST_JSON_SAMPLE: &str = r#"[
  {"href":"https:\/\/github.com\/janestreet\/magic-trace","description":"janestreet\/magic-trace: magic-trace collects and displays high-resolution traces of what a process is doing","extended":"tragically intel-only","meta":"54866bdf6b1dcbb915d917f2e2394748","hash":"54dab27be2409c987bb17fc06e47a729","time":"2022-04-23T00:29:36Z","shared":"yes","toread":"no","tags":"performance profiling tools"},
  {"href":"https:\/\/www.intel.com\/content\/www\/us\/en\/developer\/tools\/oneapi\/vtune-profiler.html#gs.x8oazh","description":"Fix Performance Bottlenecks with Intel\u00ae VTune\u2122 Profiler","extended":"","meta":"2a438b267411603d2077a26862a260e6","hash":"2ab1611711c8bb5ed9273b8f4b612fca","time":"2022-04-13T13:12:10Z","shared":"no","toread":"yes","tags":"performance profiling tools"},
  {"href":"http:\/\/kcachegrind.sourceforge.net\/html\/Home.html","description":"","extended":"","meta":"a2c175993139aed54ad3ff002439625d","hash":"0850c315d075a67430db267214b18d13","time":"2022-04-13T08:01:32Z","shared":"no","toread":"no","tags":"performance profiling tools"}
]
"#;

#[test]
fn test_json_sample() {
    let expected_tags =
        [String::from("performance"), String::from("profiling"), String::from("tools")];
    let magic_trace = Post::new(
        String::from("https://github.com/janestreet/magic-trace"),
        String::from("2022-04-23T00:29:36Z"),
        Some(String::from("janestreet/magic-trace: magic-trace collects and displays high-resolution traces of what a process is doing")),
        Some(String::from("tragically intel-only")),
        Vec::from(expected_tags.clone()),
        Some(String::from("54dab27be2409c987bb17fc06e47a729")),
        true,
        false,
    );
    let intel = Post::new(
        String::from("https://www.intel.com/content/www/us/en/developer/tools/oneapi/vtune-profiler.html#gs.x8oazh"),
        String::from("2022-04-13T13:12:10Z"),
        Some(String::from("Fix Performance Bottlenecks with Intel® VTune™ Profiler")),
        None,
        Vec::from(expected_tags.clone()),
        Some(String::from("2ab1611711c8bb5ed9273b8f4b612fca")),
        false,
        true,
    );
    let kcachegrind = Post::new(
        String::from("http://kcachegrind.sourceforge.net/html/Home.html"),
        String::from("2022-04-13T08:01:32Z"),
        None,
        None,
        Vec::from(expected_tags.clone()),
        Some(String::from("0850c315d075a67430db267214b18d13")),
        false,
        false,
    );
    let expected = vec![magic_trace, intel, kcachegrind];
    let actual = Post::from_json(TEST_JSON_SAMPLE).unwrap();
    assert_eq!(expected, actual);
    let expected_tags = Tags::new(HashSet::from(expected_tags));
    let actual_tags = Post::tags(&actual);
    assert_eq!(expected_tags, actual_tags)
}
