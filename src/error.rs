pub fn format_error_details(e: &anyhow::Error) -> String {
    // Get the full error chain
    let error_chain = e.chain().collect::<Vec<_>>();
    // Initialize result with the main error
    let mut result = format!("{}\n", e);
    // Only show the error types and unique messages
    result.push_str("Error chain:\n");

    // Track seen messages to avoid duplication
    let mut seen_messages = std::collections::HashSet::new();
    seen_messages.insert(e.to_string());

    for (i, err) in error_chain.iter().enumerate().skip(1) {
        // Extract type name (last part after ::)
        let type_name = std::any::type_name_of_val(err)
            .split("::")
            .last()
            .unwrap_or("Unknown");

        // Only add message if we haven't seen it before
        let err_msg = err.to_string();
        if seen_messages.insert(err_msg.clone()) {
            result.push_str(&format!("  [{}] {} - {}\n", i, type_name, err_msg));
        } else {
            // Just show the type if message is duplicate
            result.push_str(&format!("  [{}] {}\n", i, type_name));
        }
    }
    result
}
