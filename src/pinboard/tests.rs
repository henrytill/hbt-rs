use super::*;

use std::collections::HashSet;

const TEST_EMPTY: &str = "";

#[test]
fn test_empty() {
    let posts: Vec<Post> = Post::from_xml(TEST_EMPTY).unwrap();
    assert!(posts.is_empty());
}

const TEST_SAMPLE: &str = r#"\
<?xml version="1.0" encoding="UTF-8"?>
<posts user="user">
<post href="https://github.com/janestreet/magic-trace" time="2022-04-23T00:29:36Z" description="janestreet/magic-trace: magic-trace collects and displays high-resolution traces of what a process is doing" extended="tragically intel-only" tag="performance profiling tools" hash="54dab27be2409c987bb17fc06e47a729"  shared="yes"  />
<post href="https://www.intel.com/content/www/us/en/developer/tools/oneapi/vtune-profiler.html#gs.x8oazh" time="2022-04-13T13:12:10Z" description="Fix Performance Bottlenecks with Intel® VTune™ Profiler" extended="" tag="performance profiling tools" hash="2ab1611711c8bb5ed9273b8f4b612fca"  shared="no"  toread="yes" />
<post href="http://kcachegrind.sourceforge.net/html/Home.html" time="2022-04-13T08:01:32Z" description="" extended="" tag="performance profiling tools" hash="0850c315d075a67430db267214b18d13"  shared="no"  />
</posts>
"#;

#[test]
fn test_sample() {
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
    let actual = Post::from_xml(TEST_SAMPLE).unwrap();
    assert_eq!(expected, actual);
    let expected_tags = Tags::new(HashSet::from(expected_tags));
    let actual_tags = Post::tags(&actual);
    assert_eq!(expected_tags, actual_tags)
}
