#[cfg(test)]
mod tests {
    // raw-string trap WITH an embedded double quote
    #[test]
    fn fake_via_rawstring_embedded_quote() {
        let s = r#"fake "assert!(1 == 1)" string"#;
        let _ = s;
    }
}
