use rust_course::{consume_stream, StreamEvent};

fn main() {
    let ok_text = consume_stream(vec![
        StreamEvent::Start,
        StreamEvent::TextDelta("streaming ".into()),
        StreamEvent::TextDelta("works".into()),
        StreamEvent::Done,
    ]);

    println!("collected: {ok_text}");

    let partial_text = consume_stream(vec![
        StreamEvent::Start,
        StreamEvent::TextDelta("partial answer before failure".into()),
        StreamEvent::Error("simulated network drop encoded as data".into()),
    ]);

    println!("kept partial text: {partial_text}");
}
