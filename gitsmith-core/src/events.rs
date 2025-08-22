use anyhow::Result;
use nostr::{Event, EventBuilder, FromBech32, Keys, Kind, PublicKey, Tag, TagKind};
use std::borrow::Cow;

use crate::types::*;

pub const KIND_GIT_REPO_ANNOUNCEMENT: u16 = 30617;
pub const KIND_GIT_STATE: u16 = 30618;
pub const KIND_GIT_PATCH: u16 = 1617;

/// Build repository announcement event (NIP-34 compatible)
pub fn build_announcement_event(announcement: &RepoAnnouncement, keys: &Keys) -> Result<Event> {
    let mut tags = vec![
        Tag::identifier(&announcement.identifier),
        Tag::custom(
            TagKind::Custom(Cow::Borrowed("name")),
            vec![announcement.name.clone()],
        ),
    ];

    // Add description if not empty
    if !announcement.description.is_empty() {
        tags.push(Tag::custom(
            TagKind::Custom(Cow::Borrowed("description")),
            vec![announcement.description.clone()],
        ));
    }

    // Add root commit
    tags.push(Tag::custom(
        TagKind::Custom(Cow::Borrowed("r")),
        vec![announcement.root_commit.clone()],
    ));

    // Add clone URLs
    if !announcement.clone_urls.is_empty() {
        let mut clone_tag = vec!["clone".to_string()];
        clone_tag.extend(announcement.clone_urls.clone());
        tags.push(Tag::custom(
            TagKind::Custom(Cow::Borrowed("clone")),
            clone_tag[1..].to_vec(),
        ));
    }

    // Add relays
    for relay in &announcement.relays {
        tags.push(Tag::custom(
            TagKind::Custom(Cow::Borrowed("relays")),
            vec![relay.clone()],
        ));
    }

    // Add web URLs
    if !announcement.web.is_empty() {
        let mut web_tag = vec!["web".to_string()];
        web_tag.extend(announcement.web.clone());
        tags.push(Tag::custom(
            TagKind::Custom(Cow::Borrowed("web")),
            web_tag[1..].to_vec(),
        ));
    }

    // Add maintainers
    for maintainer in &announcement.maintainers {
        if let Ok(pubkey) = PublicKey::from_bech32(maintainer) {
            tags.push(Tag::public_key(pubkey));
        }
    }

    let event = EventBuilder::new(Kind::from(KIND_GIT_REPO_ANNOUNCEMENT), "")
        .tags(tags)
        .sign_with_keys(keys)?;

    Ok(event)
}

/// Build git state event
pub fn build_state_event(state: &GitState, keys: &Keys) -> Result<Event> {
    let mut tags = vec![Tag::identifier(&state.identifier)];

    // Add all refs
    for (ref_name, commit_hash) in &state.refs {
        tags.push(Tag::custom(
            TagKind::Custom(Cow::Owned(ref_name.clone())),
            vec![commit_hash.clone()],
        ));
    }

    // Add HEAD if present
    if let Some(head) = state.refs.get("HEAD") {
        tags.push(Tag::custom(
            TagKind::Custom(Cow::Borrowed("HEAD")),
            vec![head.clone()],
        ));
    }

    let event = EventBuilder::new(Kind::from(KIND_GIT_STATE), "")
        .tags(tags)
        .sign_with_keys(keys)?;

    Ok(event)
}
