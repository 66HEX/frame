pub(super) fn sanitized_optional_number(value: &str) -> Option<String> {
    let value: String = value.chars().filter(char::is_ascii_digit).collect();
    (!value.is_empty()).then_some(value)
}
