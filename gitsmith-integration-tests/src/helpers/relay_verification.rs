use anyhow::{Context, Result};
use nostr_sdk::{Client, EventId, Filter, Keys, RelayUrl};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};

/// Verify that an event exists on all specified relays
pub async fn verify_event_on_all_relays(
    event_id: EventId,
    relay_urls: &[String],
    max_retries: u32,
) -> Result<()> {
    info!(
        "Verifying event {} on {} relays",
        event_id,
        relay_urls.len()
    );

    for relay_url in relay_urls {
        verify_event_on_relay(event_id, relay_url, max_retries)
            .await
            .with_context(|| format!("Failed to verify event on relay {}", relay_url))?;
    }

    info!(
        "Event {} verified on all {} relays",
        event_id,
        relay_urls.len()
    );
    Ok(())
}

/// Verify that an event exists on a specific relay
pub async fn verify_event_on_relay(
    event_id: EventId,
    relay_url: &str,
    max_retries: u32,
) -> Result<()> {
    debug!("Verifying event {} on relay {}", event_id, relay_url);

    // Create a client for verification
    let keys = Keys::generate();
    let client = Client::new(keys);

    // Add only the specific relay
    let url = RelayUrl::parse(relay_url)?;
    client.add_relay(url.clone()).await?;
    client.connect().await;

    // Wait for connection
    sleep(Duration::from_millis(500)).await;

    // Create filter for the specific event
    let filter = Filter::new().id(event_id);

    // Retry with exponential backoff
    let mut delay_ms = 100u64;
    for attempt in 0..=max_retries {
        debug!(
            "Attempt {}/{} to verify event on {}",
            attempt + 1,
            max_retries + 1,
            relay_url
        );

        // Query the relay
        let events = client
            .fetch_events_from(vec![url.clone()], filter.clone(), Duration::from_secs(10))
            .await?;

        let events_vec: Vec<_> = events.into_iter().collect();
        if !events_vec.is_empty() {
            debug!("Event {} found on relay {}", event_id, relay_url);
            client.disconnect().await;
            return Ok(());
        }

        if attempt < max_retries {
            debug!("Event not found yet, waiting {}ms before retry", delay_ms);
            sleep(Duration::from_millis(delay_ms)).await;
            delay_ms = (delay_ms * 2).min(5000); // Cap at 5 seconds
        }
    }

    client.disconnect().await;
    anyhow::bail!(
        "Event {} not found on relay {} after {} retries",
        event_id,
        relay_url,
        max_retries
    )
}
