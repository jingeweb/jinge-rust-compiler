use super::*;

#[test]
fn test_trim_html_text() {
  let mut t = Wtf8Atom::from("  hello  ");
  assert_eq!(trim_html_text(&t), Some(t));
  t = Wtf8Atom::from("   \n   hello  ");
  assert_eq!(trim_html_text(&t), Some(Wtf8Atom::from("hello  ")));
  t = Wtf8Atom::from("   \n   he  llo  \n   ");
  assert_eq!(trim_html_text(&t), Some(Wtf8Atom::from("he  llo")));
  t = Wtf8Atom::from(" he llo \n w \n  orld");
  assert_eq!(trim_html_text(&t), Some(Wtf8Atom::from(" he llo w orld")));
  t = Wtf8Atom::from("  \n  \n\n  ");
  assert_eq!(trim_html_text(&t), None);
  t = Wtf8Atom::from("  a \n\n\n b c \n d ");
  assert_eq!(trim_html_text(&t), Some(Wtf8Atom::from("  a b c d ")));
}

#[test]
fn test_trim_html_text_with_multibyte_chars() {
  // 多字节字符不能按「字符下标」当作字节偏移来切片。
  let mut t = Wtf8Atom::from("\n  通过\n");
  assert_eq!(trim_html_text(&t), Some(Wtf8Atom::from("通过")));
  t = Wtf8Atom::from("\n  你好 \n  世界  ");
  assert_eq!(trim_html_text(&t), Some(Wtf8Atom::from("你好 世界  ")));
  t = Wtf8Atom::from("\n  你好  世界\n");
  assert_eq!(trim_html_text(&t), Some(Wtf8Atom::from("你好  世界")));
  t = Wtf8Atom::from("  普通文本  ");
  assert_eq!(trim_html_text(&t), Some(t));
}
