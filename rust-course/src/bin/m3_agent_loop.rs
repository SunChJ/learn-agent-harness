use rust_course::{
    print_history, run_agent_loop, EchoTool, ModelStep, ScriptedModel, ShellTool, ToolCall,
};

fn main() {
    let model = ScriptedModel::new([
        ModelStep::tool_request(
            "I should inspect the workspace before answering.",
            vec![ToolCall::new("call_1", "bash", "pwd")],
        ),
        ModelStep::tool_request(
            "Now I can echo a stable result.",
            vec![ToolCall::new("call_2", "echo", "agent loop reached Rust")],
        ),
        ModelStep::text("Done: the model stopped asking for tools, so the turn is complete."),
    ]);

    let history = run_agent_loop(
        model,
        vec![Box::new(ShellTool::new(["pwd"])), Box::new(EchoTool)],
        "Show me the minimal harness loop.",
    );

    print_history(&history);
}
