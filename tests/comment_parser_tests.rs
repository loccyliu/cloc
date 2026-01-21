use cloc::comment_parser::{
    classify_line_c_like, classify_line_css_like, classify_line_lua_like, classify_line_python_like,
    classify_line_xml_like, LuaState, ParseState, PythonState,
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

