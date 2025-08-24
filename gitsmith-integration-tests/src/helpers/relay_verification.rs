use anyhow::{Context, Result};
use nostr_sdk::{Client, EventId, Filter, Keys, Kind, PublicKey, RelayUrl};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

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

/// Verify that multiple events exist on all relays
pub async fn verify_events_on_all_relays(
    event_ids: &[EventId],
    relay_urls: &[String],
    max_retries: u32,
) -> Result<()> {
    info!(
        "Verifying {} events on {} relays",
        event_ids.len(),
        relay_urls.len()
    );

    for event_id in event_ids {
        verify_event_on_all_relays(*event_id, relay_urls, max_retries).await?;
    }

    Ok(())
}

/// Verify that a repository announcement exists on all relays
pub async fn verify_repo_announcement_on_all_relays(
    identifier: &str,
    author_pubkey: &str,
    relay_urls: &[String],
    max_retries: u32,
) -> Result<EventId> {
    info!(
        "Verifying repo announcement '{}' on {} relays",
        identifier,
        relay_urls.len()
    );

    // Parse author public key
    let author = PublicKey::from_hex(author_pubkey)?;

    // Create filter for repository announcement (Kind 30617)
    let filter = Filter::new()
        .kind(Kind::from(30617))
        .author(author)
        .identifier(identifier);

    let mut found_event_id = None;

    for relay_url in relay_urls {
        debug!("Checking relay {} for repo announcement", relay_url);

        // Create a client for verification
        let keys = Keys::generate();
        let client = Client::new(keys);

        // Add only the specific relay
        let url = RelayUrl::parse(relay_url)?;
        client.add_relay(url.clone()).await?;
        client.connect().await;

        // Wait for connection
        sleep(Duration::from_millis(500)).await;

        // Retry with exponential backoff
        let mut delay_ms = 100u64;
        let mut found = false;

        for attempt in 0..=max_retries {
            debug!(
                "Attempt {}/{} to find repo announcement on {}",
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
                let event = &events_vec[0];
                debug!(
                    "Repo announcement found on relay {}: {}",
                    relay_url, event.id
                );

                if found_event_id.is_none() {
                    found_event_id = Some(event.id);
                } else if found_event_id != Some(event.id) {
                    warn!("Different event IDs found on different relays!");
                }

                found = true;
                break;
            }

            if attempt < max_retries {
                debug!(
                    "Repo announcement not found yet, waiting {}ms before retry",
                    delay_ms
                );
                sleep(Duration::from_millis(delay_ms)).await;
                delay_ms = (delay_ms * 2).min(5000); // Cap at 5 seconds
            }
        }

        client.disconnect().await;

        if !found {
            anyhow::bail!(
                "Repository announcement '{}' not found on relay {} after {} retries",
                identifier,
                relay_url,
                max_retries
            );
        }
    }

    found_event_id.ok_or_else(|| anyhow::anyhow!("No event ID found"))
}

/// Check relay connectivity
pub async fn check_relay_connectivity(relay_url: &str) -> Result<bool> {
    debug!("Checking connectivity to relay {}", relay_url);

    let keys = Keys::generate();
    let client = Client::new(keys);

    let url = RelayUrl::parse(relay_url)?;
    match client.add_relay(url.clone()).await {
        Ok(_) => {
            client.connect().await;
            sleep(Duration::from_millis(500)).await;

            // Try to get relay information to check if connected
            // We'll use a simple query with an impossible filter to test connectivity
            let test_filter = Filter::new().id(EventId::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )?);

            let connected = match client
                .fetch_events_from(vec![url.clone()], test_filter, Duration::from_secs(5))
                .await
            {
                Ok(_) => {
                    debug!("Successfully connected to relay {}", relay_url);
                    true
                }
                Err(e) => {
                    warn!("Failed to connect to relay {}: {}", relay_url, e);
                    false
                }
            };

            client.disconnect().await;
            Ok(connected)
        }
        Err(e) => {
            warn!("Failed to add relay {}: {}", relay_url, e);
            Ok(false)
        }
    }
}

/// Verify all relays are accessible
pub async fn verify_all_relays_accessible(relay_urls: &[String]) -> Result<()> {
    info!("Verifying connectivity to {} relays", relay_urls.len());

    let mut failed_relays = Vec::new();

    for relay_url in relay_urls {
        if !check_relay_connectivity(relay_url).await? {
            failed_relays.push(relay_url.clone());
        }
    }

    if !failed_relays.is_empty() {
        anyhow::bail!(
            "Failed to connect to {} relay(s): {:?}",
            failed_relays.len(),
            failed_relays
        );
    }

    info!("All {} relays are accessible", relay_urls.len());
    Ok(())
}
