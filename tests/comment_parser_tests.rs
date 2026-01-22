use cloc::comment_parser::{
    classify_line_batch_like,
    classify_line_c_like, classify_line_css_like, classify_line_lua_like, classify_line_python_like,
    classify_line_xml_like, classify_line_sql_like,
    LuaState, ParseState, PythonState,
};

#[test]
fn c_like_inline_block_comment_code_both_sides() {
    let mut st = ParseState::new();
    let (code, comment) = classify_line_c_like("let x = 1; /* hi */ let y = 2;", &mut st);
    assert!(code);
    assert!(comment);
    assert!(!st.in_block_comment);
}

#[test]
fn c_like_multiline_block_comment() {
    let mut st = ParseState::new();
    let (c1, m1) = classify_line_c_like("/* start", &mut st);
    assert!(!c1);
    assert!(m1);
    assert!(st.in_block_comment);

    let (c2, m2) = classify_line_c_like("middle", &mut st);
    assert!(!c2);
    assert!(m2);
    assert!(st.in_block_comment);

    let (c3, m3) = classify_line_c_like("end */ let z=1;", &mut st);
    assert!(c3);
    assert!(m3);
    assert!(!st.in_block_comment);
}

#[test]
fn c_like_trailing_line_comment() {
    let mut st = ParseState::new();
    let (code, comment) = classify_line_c_like("let x = 1; // trailing", &mut st);
    assert!(code);
    assert!(comment);
}

#[test]
fn c_like_ignore_comment_markers_inside_strings() {
    let mut st = ParseState::new();
    let (code, comment) = classify_line_c_like("let s = \"http://a//b\";", &mut st);
    assert!(code);
    assert!(!comment);
}

#[test]
fn python_hash_comment_after_code() {
    let mut st = PythonState::new();
    let (code, comment) = classify_line_python_like("x = 1  # hi", &mut st);
    assert!(code);
    assert!(comment);
}

#[test]
fn python_triple_quote_block() {
    let mut st = PythonState::new();
    let (c1, m1) = classify_line_python_like("\"\"\"doc", &mut st);
    assert!(!c1);
    assert!(m1);
    assert!(st.in_triple.is_some());

    let (c2, m2) = classify_line_python_like("inside", &mut st);
    assert!(!c2);
    assert!(m2);

    let (c3, m3) = classify_line_python_like("end\"\"\" x = 1", &mut st);
    assert!(c3);
    assert!(m3);
    assert!(st.in_triple.is_none());
}

#[test]
fn lua_line_comment_and_code() {
    let mut st = LuaState::new();
    let (code, comment) = classify_line_lua_like("local x = 1 -- trailing", &mut st);
    assert!(code);
    assert!(comment);
}

#[test]
fn xml_inline_comment() {
    let mut st = ParseState::new();
    let (code, comment) = classify_line_xml_like("<a><!--c--></a>", &mut st);
    assert!(code);
    assert!(comment);
}

#[test]
fn css_inline_comment() {
    let mut st = ParseState::new();
    let (code, comment) = classify_line_css_like("a{/*c*/color:red}", &mut st);
    assert!(code);
    assert!(comment);
}

#[test]
fn hash_comment_ignore_inside_quotes_common_configs() {
    let mut st = PythonState::new();
    let (code, comment) = classify_line_python_like("k = \"a#b\"", &mut st);
    assert!(code);
    assert!(!comment);
}

#[test]
fn hash_comment_code_and_comment_same_line_yaml_style() {
    let mut st = PythonState::new();
    let (code, comment) = classify_line_python_like("key: 1  # hi", &mut st);
    assert!(code);
    assert!(comment);
}

#[test]
fn jsonc_like_trailing_line_comment() {
    let mut st = ParseState::new();
    let (code, comment) = classify_line_c_like("{\"a\": 1} // hi", &mut st);
    assert!(code);
    assert!(comment);
}

#[test]
fn json_string_with_double_slash_is_not_comment() {
    let mut st = ParseState::new();
    let (code, comment) = classify_line_c_like("{\"url\": \"http://a//b\"}", &mut st);
    assert!(code);
    assert!(!comment);
}

#[test]
fn markdown_multiline_html_comment() {
    let mut st = ParseState::new();
    let (c1, m1) = classify_line_xml_like("<!-- start", &mut st);
    assert!(!c1);
    assert!(m1);
    assert!(st.in_block_comment);

    let (c2, m2) = classify_line_xml_like("middle", &mut st);
    assert!(!c2);
    assert!(m2);
    assert!(st.in_block_comment);

    let (c3, m3) = classify_line_xml_like("end --> text", &mut st);
    assert!(c3);
    assert!(m3);
    assert!(!st.in_block_comment);
}

#[test]
fn batch_rem_and_colon_colon_comments() {
    assert_eq!(classify_line_batch_like("REM hello"), (false, true));
    assert_eq!(classify_line_batch_like("   rem\tHello"), (false, true));
    assert_eq!(classify_line_batch_like(":: hello"), (false, true));
    assert_eq!(classify_line_batch_like("echo REM hello"), (true, false));
    assert_eq!(classify_line_batch_like("set X=1"), (true, false));
}

#[test]
fn kts_uses_c_like_comments() {
    let mut st = ParseState::new();
    let (code, comment) = classify_line_c_like("val x = 1 // hi", &mut st);
    assert!(code);
    assert!(comment);
}

#[test]
fn sql_trailing_line_comment() {
    let mut st = ParseState::new();
    let (code, comment) = classify_line_sql_like("select 1 -- hi", &mut st);
    assert!(code);
    assert!(comment);
}

#[test]
fn sql_multiline_block_comment() {
    let mut st = ParseState::new();
    let (c1, m1) = classify_line_sql_like("/* start", &mut st);
    assert!(!c1);
    assert!(m1);
    assert!(st.in_block_comment);

    let (c2, m2) = classify_line_sql_like("middle", &mut st);
    assert!(!c2);
    assert!(m2);
    assert!(st.in_block_comment);

    let (c3, m3) = classify_line_sql_like("end */ select 1", &mut st);
    assert!(c3);
    assert!(m3);
    assert!(!st.in_block_comment);
}

#[test]
fn sql_string_with_double_dash_is_not_comment() {
    let mut st = ParseState::new();
    let (code, comment) = classify_line_sql_like("select '--not comment'", &mut st);
    assert!(code);
    assert!(!comment);
}
