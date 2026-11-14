//! Server-sent event frames that tell `data: [DONE]` apart from the end of the body.
//!
//! [`timvw_rust_runtime::sse::Events`] ends its stream on either; the chat completion decoder
//! needs to know which one it saw (a stream that ends without `[DONE]` and without a
//! `finish_reason` was cut off), so this reads the body with the runtime's [`sse::Decoder`].

use crate::{BoxStream, MaybeSend};
use futures::stream::{Stream, StreamExt};
use std::collections::VecDeque;
use timvw_rust_runtime::sse;

/// One frame of a `text/event-stream` body.
pub enum Frame {
    /// An event with a `data:` payload other than `[DONE]`.
    Event(sse::Event),
    /// The `data: [DONE]` sentinel; nothing follows it.
    Done,
}

/// The frames of a body read as a stream of chunks. The stream ends after [`Frame::Done`], at
/// the end of the body (an incomplete trailing event is discarded), or after the first read
/// error; the body is dropped then.
pub fn frames<S, B>(bytes: S) -> BoxStream<'static, Result<Frame, timvw_rust_runtime::Error>>
where
    S: Stream<Item = Result<B, timvw_rust_runtime::Error>> + MaybeSend + 'static,
    B: AsRef<[u8]> + MaybeSend + 'static,
{
    struct State<S> {
        bytes: Option<std::pin::Pin<Box<S>>>,
        decoder: sse::Decoder,
        ready: VecDeque<sse::Event>,
    }

    let state = State {
        bytes: Some(Box::pin(bytes)),
        decoder: sse::Decoder::new(),
        ready: VecDeque::new(),
    };
    crate::box_stream(futures::stream::unfold(state, |mut state| async move {
        loop {
            if let Some(event) = state.ready.pop_front() {
                if event.data == sse::DONE {
                    state.ready.clear();
                    state.bytes = None;
                    return Some((Ok(Frame::Done), state));
                }
                return Some((Ok(Frame::Event(event)), state));
            }
            let bytes = state.bytes.as_mut()?;
            match bytes.next().await {
                Some(Ok(chunk)) => state.decoder.feed(chunk.as_ref(), &mut state.ready),
                Some(Err(error)) => {
                    state.bytes = None;
                    state.decoder.finish();
                    return Some((Err(error), state));
                }
                None => {
                    state.bytes = None;
                    state.decoder.finish();
                }
            }
        }
    }))
}

/// The frames of events parsed elsewhere: `[DONE]` cannot be seen there, as `Events` ends the
/// stream on it without yielding it.
pub fn from_events(
    events: sse::Events,
) -> BoxStream<'static, Result<Frame, timvw_rust_runtime::Error>> {
    crate::box_stream(events.map(|event| event.map(Frame::Event)))
}
