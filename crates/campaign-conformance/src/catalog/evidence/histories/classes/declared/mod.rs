//! Declared forbidden effects and Campaign's answers of the forged and suffix catalog classes.

/// Forbidden effects of loading, and of using a cache of, a history of each class.
const EFFECTS: [(super::Class, super::super::shapes::Names); 22] = [
    (
        super::Class::Renamed,
        (
            "a renamed control branch was loaded as the campaign's history",
            "a cache of a renamed control branch was used as current state",
        ),
    ),
    (
        super::Class::Reset,
        (
            "a control branch reset onto an earlier head was loaded as current history",
            "a cache of a branch reset onto an earlier head was used as current state",
        ),
    ),
    (
        super::Class::Lagging,
        (
            "control payloads lagging the branch were loaded as current history",
            "a cache was validated against payloads lagging the branch",
        ),
    ),
    (
        super::Class::RewrittenTail,
        (
            "a control payload rewritten into another replayable event was loaded as history",
            "a cache of a rewritten control payload that replays was used as current state",
        ),
    ),
    (
        super::Class::RewrittenFirst,
        (
            "a rewritten control payload was loaded as history",
            "a cache of a rewritten control payload was used as current state",
        ),
    ),
    (
        super::Class::Altered,
        (
            "history altered after binding was loaded as current",
            "a cache of history altered after binding was used as current state",
        ),
    ),
    (
        super::Class::Ambiguous,
        (
            "an ambiguous control lineage was loaded as history",
            "a cache of an ambiguous control lineage was used as current state",
        ),
    ),
    (
        super::Class::TwoParentUnchosen,
        (
            "a merged control event was loaded without a parent choice",
            "a cache of a merged control event without a parent choice was used",
        ),
    ),
    (
        super::Class::TwoParentControl,
        (
            "a merged control event was loaded as the linear child of the head",
            "a cache of a merged control event with its control parent chosen was used",
        ),
    ),
    (
        super::Class::TwoParentForeign,
        (
            "a merged control event was loaded along its foreign parent",
            "a cache of a merged control event with its foreign parent chosen was used",
        ),
    ),
    (
        super::Class::TwoParentNeither,
        (
            "a merged control event was loaded along a parent choice naming neither parent",
            "a cache of a merged control event with a choice naming neither parent was used",
        ),
    ),
    (
        super::Class::ForgedFence,
        (
            "a selection recorded with a forged publication fence was loaded as history",
            "a cache of a history with a forged publication fence was used as current state",
        ),
    ),
    (
        super::Class::Stale,
        (
            "a control event Campaign refuses was loaded as history",
            "a cache of a history with a stale-state event was used as current state",
        ),
    ),
    (
        super::Class::WorkAfterStop,
        (
            "new work after an accepted stop was loaded as history",
            "a cache of a history with new work after a stop was used as current state",
        ),
    ),
    (
        super::Class::Replayed,
        (
            "a replayed control event was loaded as new history",
            "a cache of a history with a replayed control event was used as current state",
        ),
    ),
    (
        super::Class::ForeignMetadata,
        (
            "a control event carrying another adapter's metadata was loaded as history",
            "a cache of a history with another adapter's metadata was used as current state",
        ),
    ),
    (
        super::Class::NonApplication,
        (
            "a non-application control envelope was loaded as history",
            "a cache of a history with a non-application envelope was used as current state",
        ),
    ),
    (
        super::Class::ForeignSchema,
        (
            "a control event under a foreign schema was loaded as history",
            "a cache of a history with a foreign schema was used as current state",
        ),
    ),
    (
        super::Class::ForeignProjection,
        (
            "a control event under a foreign projection identity was loaded as history",
            "a cache of a history with a foreign projection identity was used as current state",
        ),
    ),
    (
        super::Class::ProjectionLimit,
        (
            "a control lineage longer than the projection bound was loaded as history",
            "a cache of a control lineage longer than the projection bound was used as current \
             state",
        ),
    ),
    (
        super::Class::SharedGraph,
        (
            "a control lineage on a graph holding another campaign's root was loaded as history",
            "a cache of a control lineage on a graph holding another campaign's root was used as \
             current state",
        ),
    ),
    (
        super::Class::ForeignEnvelopeRoot,
        (
            "a control root under foreign envelope identities was loaded as the campaign's \
             history",
            "a cache of a control root under foreign envelope identities was used as current \
             state",
        ),
    ),
];

/// Refusal with one adapter error.
const fn refused(error: campaign_choregraph::AdapterError) -> super::super::Answer {
    super::super::Answer::Refused(error)
}

/// Refusal with one Campaign error.
const fn campaign(error: campaign_core::CampaignError) -> super::super::Answer {
    refused(campaign_choregraph::AdapterError::Campaign(error))
}

/// Campaign's answer to a history of each class whose answer carries none of the history's own
/// facts; the merge naming neither parent, the projection limit, and the shared graph declare
/// theirs where the history is built.
const ANSWERS: [(super::Class, super::super::Answer); 19] = [
    (
        super::Class::Renamed,
        refused(campaign_choregraph::AdapterError::ControlBranchMismatch),
    ),
    (super::Class::Reset, super::super::shapes::NOT_FORWARD),
    (
        super::Class::Lagging,
        refused(campaign_choregraph::AdapterError::ProjectionPayloadMismatch),
    ),
    (
        super::Class::RewrittenTail,
        refused(campaign_choregraph::AdapterError::PayloadIdentityMismatch),
    ),
    (
        super::Class::RewrittenFirst,
        refused(campaign_choregraph::AdapterError::PayloadIdentityMismatch),
    ),
    (
        super::Class::Altered,
        refused(campaign_choregraph::AdapterError::ProjectionBindingMismatch),
    ),
    (super::Class::Ambiguous, super::super::Answer::Ambiguous),
    (
        super::Class::TwoParentUnchosen,
        super::super::Answer::Ambiguous,
    ),
    (
        super::Class::TwoParentControl,
        refused(campaign_choregraph::AdapterError::ParentLineageMismatch),
    ),
    (
        super::Class::TwoParentForeign,
        refused(campaign_choregraph::AdapterError::ProjectionPayloadMismatch),
    ),
    (
        super::Class::ForgedFence,
        refused(campaign_choregraph::AdapterError::PublicationFenceMismatch),
    ),
    (
        super::Class::Stale,
        campaign(campaign_core::CampaignError::StaleState),
    ),
    (
        super::Class::WorkAfterStop,
        campaign(campaign_core::CampaignError::CampaignStopped),
    ),
    (
        super::Class::Replayed,
        campaign(campaign_core::CampaignError::DuplicateControlEvent),
    ),
    (
        super::Class::ForeignMetadata,
        refused(campaign_choregraph::AdapterError::MetadataMismatch),
    ),
    (
        super::Class::NonApplication,
        refused(campaign_choregraph::AdapterError::NonApplicationEnvelope),
    ),
    (
        super::Class::ForeignSchema,
        refused(campaign_choregraph::AdapterError::SchemaIdentityMismatch),
    ),
    (
        super::Class::ForeignProjection,
        refused(campaign_choregraph::AdapterError::ProjectionIdentityMismatch),
    ),
    (
        super::Class::ForeignEnvelopeRoot,
        campaign(campaign_core::CampaignError::CrossedIdentity),
    ),
];

/// The forbidden effects of one class.
///
/// # Errors
///
/// Returns `Diverged` for a class without declared effects.
pub(super) fn effects(
    class: super::Class,
) -> crate::verdicts::ConformanceResult<super::super::shapes::Names> {
    let declared = EFFECTS
        .iter()
        .find(|(declared, _)| *declared == class)
        .map(|(_, effects)| *effects);
    debug_assert!(EFFECTS.iter().all(|(_, (loaded, cached))| loaded != cached));
    debug_assert_eq!(EFFECTS.len(), super::CLASSES.len());
    declared.ok_or(crate::verdicts::ConformanceError::Diverged {
        expected: "every catalog class declares its forbidden effects",
    })
}

/// Campaign's answer to a history of one class, when it carries none of the history's facts.
pub(super) fn answer(class: super::Class) -> Option<super::super::Answer> {
    let declared = ANSWERS
        .iter()
        .find(|(declared, _)| *declared == class)
        .map(|(_, answer)| answer.clone());
    debug_assert!(ANSWERS.len() < super::CLASSES.len());
    debug_assert!(ANSWERS.iter().all(|(declared, _)| {
        !matches!(
            declared,
            super::Class::TwoParentNeither
                | super::Class::ProjectionLimit
                | super::Class::SharedGraph
        )
    }));
    declared
}
