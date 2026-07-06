use std::collections::{HashMap, VecDeque};
use std::process::Command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Role {
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: String,
}

impl ToolCall {
    pub fn new(id: impl Into<String>, name: impl Into<String>, input: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            input: input.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelStep {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
}

impl ModelStep {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            text: content.into(),
            tool_calls: Vec::new(),
        }
    }

    pub fn tool_request(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            text: content.into(),
            tool_calls,
        }
    }
}

pub struct ScriptedModel {
    steps: VecDeque<ModelStep>,
}

impl ScriptedModel {
    pub fn new(steps: impl IntoIterator<Item = ModelStep>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
        }
    }

    pub fn sample(&mut self, _history: &[Message]) -> ModelStep {
        self.steps
            .pop_front()
            .unwrap_or_else(|| ModelStep::text("No more scripted model steps."))
    }
}

pub trait Tool {
    fn name(&self) -> &'static str;
    fn execute(&self, input: &str) -> String;
}

pub struct EchoTool;

impl Tool for EchoTool {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn execute(&self, input: &str) -> String {
        input.to_owned()
    }
}

pub struct ShellTool {
    allowed: Vec<String>,
}

impl ShellTool {
    pub fn new(allowed: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            allowed: allowed.into_iter().map(Into::into).collect(),
        }
    }
}

impl Tool for ShellTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn execute(&self, input: &str) -> String {
        if !self.allowed.iter().any(|allowed| allowed == input) {
            return format!("blocked: command is not in the lab allowlist: {input}");
        }

        let output = Command::new("sh").arg("-c").arg(input).output();
        match output {
            Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
                .trim_end()
                .to_owned(),
            Ok(output) => String::from_utf8_lossy(&output.stderr)
                .trim_end()
                .to_owned(),
            Err(err) => format!("failed to spawn shell: {err}"),
        }
    }
}

pub fn run_agent_loop(
    mut model: ScriptedModel,
    tools: Vec<Box<dyn Tool>>,
    user_input: &str,
) -> Vec<Message> {
    let mut history = vec![Message::user(user_input)];
    let tools_by_name: HashMap<&'static str, Box<dyn Tool>> =
        tools.into_iter().map(|tool| (tool.name(), tool)).collect();

    loop {
        let step = model.sample(&history);
        history.push(Message::assistant(step.text));

        if step.tool_calls.is_empty() {
            break;
        }

        for call in step.tool_calls {
            let result = match tools_by_name.get(call.name.as_str()) {
                Some(tool) => tool.execute(&call.input),
                None => format!("unknown tool: {}", call.name),
            };
            history.push(Message::tool(format!(
                "tool_call_id={} name={} result={}",
                call.id, call.name, result
            )));
        }
    }

    history
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamEvent {
    Start,
    TextDelta(String),
    Done,
    Error(String),
}

pub fn consume_stream(events: Vec<StreamEvent>) -> String {
    let mut text = String::new();
    for event in events {
        match event {
            StreamEvent::Start => println!("stream started"),
            StreamEvent::TextDelta(delta) => {
                print!("{delta}");
                text.push_str(&delta);
            }
            StreamEvent::Done => {
                println!("\nstream done");
                break;
            }
            StreamEvent::Error(message) => {
                println!("\nstream error event: {message}");
                break;
            }
        }
    }
    text
}

pub fn compact_keep_recent_turns(history: &[Message], keep: usize) -> Vec<Message> {
    if history.len() <= keep {
        return history.to_vec();
    }

    let mut start = history.len() - keep;
    while start < history.len() && matches!(history[start].role, Role::Tool) {
        start = start.saturating_sub(1);
    }

    let summarized = history[..start]
        .iter()
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join(" | ");

    let mut compacted = vec![Message::user(format!("[compact summary] {summarized}"))];
    compacted.extend_from_slice(&history[start..]);
    compacted
}

pub fn print_history(history: &[Message]) {
    for (index, message) in history.iter().enumerate() {
        println!("{index:02} {:?}: {}", message.role, message.content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_loop_stops_when_model_returns_plain_text() {
        let history = run_agent_loop(
            ScriptedModel::new([
                ModelStep::tool_request(
                    "I need the tool.",
                    vec![ToolCall::new("call_1", "echo", "hello")],
                ),
                ModelStep::text("Final answer."),
            ]),
            vec![Box::new(EchoTool)],
            "Say hello",
        );

        assert_eq!(history.last().unwrap().content, "Final answer.");
        assert!(history
            .iter()
            .any(|message| message.content.contains("hello")));
    }

    #[test]
    fn compaction_does_not_start_with_orphan_tool_result() {
        let history = vec![
            Message::user("u1"),
            Message::assistant("a1 tool call"),
            Message::tool("tool result"),
            Message::assistant("a2"),
        ];

        let compacted = compact_keep_recent_turns(&history, 2);
        assert!(!matches!(compacted[1].role, Role::Tool));
    }
}
