use rust_course::{compact_keep_recent_turns, print_history, Message};

fn main() {
    let history = vec![
        Message::user("Please inspect the project."),
        Message::assistant("I will call bash."),
        Message::tool("tool_call_id=1 name=bash result=README.md docs rust-course"),
        Message::user("Remember that rust-course exists."),
        Message::assistant("Noted. I will keep the Rust course in context."),
        Message::user("Now continue after compaction."),
    ];

    println!("before compaction:");
    print_history(&history);

    let compacted = compact_keep_recent_turns(&history, 3);

    println!("\nafter compaction:");
    print_history(&compacted);
}
