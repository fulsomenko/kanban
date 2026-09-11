use crate::state::AppState;
use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::Router;
use futures_util::stream;
use kanban_service::api::ChangeEventFrame;
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast;

fn frame_to_event(frame: &ChangeEventFrame) -> Event {
    Event::default()
        .json_data(frame)
        .expect("ChangeEventFrame always serializes")
}

async fn events(
    State(state): State<AppState>,
) -> Sse<impl futures_util::stream::Stream<Item = Result<Event, Infallible>> + Send> {
    let rx = state.event_tx.subscribe();
    let stream = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(frame) => return Some((Ok(frame_to_event(&frame)), rx)),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

pub fn router() -> Router<AppState> {
    Router::new().route("/v1/events", get(events))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kanban_core::ClientId;
    use uuid::Uuid;

    #[test]
    fn test_change_event_frame_maps_to_sse_data_json() {
        let frame = ChangeEventFrame::now(
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4(),
            kanban_core::ClientId::new(),
        );
        let event = frame_to_event(&frame);
        let _ = event;
    }

    #[tokio::test]
    async fn test_lagged_subscriber_receives_synthetic_all_frame() {
        let (tx, mut rx) = broadcast::channel::<ChangeEventFrame>(2);
        let instance_id = Uuid::new_v4();
        let oldest_retained_correlation_id = Uuid::from_bytes([2; 16]);

        for i in 0..4u8 {
            let _ = tx.send(ChangeEventFrame::now(
                instance_id,
                Uuid::from_bytes([i; 16]),
                ClientId::new(),
            ));
        }

        let frame = next_event_frame(&mut rx, instance_id).await;
        let frame = frame.expect("expected a synthetic frame after lag");
        assert!(frame.invalidation.is_none());
        assert_eq!(frame.issued_by, ClientId::nil());
        assert_eq!(frame.writer_instance_id, instance_id);

        let next = next_event_frame(&mut rx, instance_id)
            .await
            .expect("expected the oldest retained real frame");
        assert_eq!(next.correlation_id, oldest_retained_correlation_id);
        assert_ne!(next.issued_by, ClientId::nil());
    }
}
