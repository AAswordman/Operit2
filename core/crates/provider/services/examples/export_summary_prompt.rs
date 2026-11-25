use operit_providers::chat::config::FunctionalPrompts::FunctionalPrompts;

/// Prints the production summary directive for live prompt-quality verification.
fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let english = arguments.iter().any(|argument| argument == "--english");
    if arguments.iter().any(|argument| argument == "--template-only") {
        print!("{}", FunctionalPrompts::summaryPrompt(english));
    } else {
        print!("{}", FunctionalPrompts::summaryContinuationInstruction(english));
    }
}
