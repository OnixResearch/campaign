//! Oracle equality of the consumer shell.
//!
//! Wherever the suite asks the adapter's shell a question Campaign also answers (history load,
//! cache validation, execution gate), the shell's answer must equal Campaign's own answer for the
//! exact presented input. The session computes Campaign's answer itself and fails the fixture on
//! any difference, so a shell that only reproduces Campaign's answers on the inputs a fixture
//! happens to check cannot conform.

pub(crate) mod hosted;

/// Expectation named when the shell refuses an input Campaign accepts.
const SHELL_REFUSED: &str = "the adapter's shell accepts exactly what Campaign accepts";

/// Expectation named when the shell and Campaign accept an input with different values.
const SHELL_REPLACED: &str = "the adapter's shell returns Campaign's own answer";

/// Compares the shell's answer with Campaign's answer for the same input and returns the agreed
/// answer.
///
/// # Errors
///
/// Returns `ForbiddenAccepted` when the shell accepts what Campaign refuses, `UndeclaredDenial`
/// when both refuse with different errors, and `Diverged` when the shell refuses what Campaign
/// accepts or accepts with another value.
pub(crate) fn agree<T: PartialEq>(
    shell: campaign_choregraph::AdapterResult<T>,
    campaign: &campaign_choregraph::AdapterResult<T>,
    forbidden: &'static str,
) -> crate::verdicts::ConformanceResult<campaign_choregraph::AdapterResult<T>> {
    if shell == *campaign {
        return Ok(shell);
    }
    debug_assert!(shell != *campaign);
    debug_assert!(!forbidden.is_empty());
    match shell {
        Ok(_) if campaign.is_err() => {
            Err(crate::verdicts::ConformanceError::ForbiddenAccepted { forbidden })
        }
        Ok(_) => Err(crate::verdicts::ConformanceError::Diverged {
            expected: SHELL_REPLACED,
        }),
        Err(_) if campaign.is_ok() => Err(crate::verdicts::ConformanceError::Diverged {
            expected: SHELL_REFUSED,
        }),
        Err(denial) => Err(crate::verdicts::ConformanceError::UndeclaredDenial {
            forbidden,
            denial: alloc::boxed::Box::new(crate::verdicts::ConformanceError::Adapter(denial)),
        }),
    }
}

/// Campaign's own load answer: a fast-forward of `previous` when the host holds one, otherwise
/// the cold-start validation.
pub(crate) fn campaign_load(
    request: &campaign_choregraph::ControlProjectionRequest<'_>,
    previous: Option<&campaign_choregraph::ValidatedControlProjection>,
) -> campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection> {
    match previous {
        Some(previous) => campaign_choregraph::validate_control_advance(previous, request),
        None => campaign_choregraph::validate_control_projection(request),
    }
}

/// An owned copy of one history value (graph, control branch, parent choices, and control
/// payloads).
#[derive(Clone, Debug)]
pub(crate) struct HistoryView {
    pub(crate) graph: campaign_choregraph::HistoryGraph,
    pub(crate) branch: campaign_choregraph::BranchRef,
    pub(crate) parent_choices: campaign_choregraph::ParentChoices,
    pub(crate) payloads: alloc::vec::Vec<campaign_choregraph::ControlPayloadEnvelope>,
}

/// How the host presents a history to the adapter's load path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Presentation {
    /// Cold start: the host holds no validated projection.
    Cold,
    /// Through the projection the session last validated.
    ThroughHeld,
}

/// Both presentations, cold first.
pub(crate) const PRESENTATIONS: [Presentation; 2] = [Presentation::Cold, Presentation::ThroughHeld];

impl super::Session<'_, '_> {
    /// Owned copy of the session's current history.
    pub(crate) fn history(&self) -> HistoryView {
        HistoryView {
            graph: self.graph.clone(),
            branch: self.branch.clone(),
            parent_choices: campaign_choregraph::ParentChoices::new(),
            payloads: self.payloads.clone(),
        }
    }

    /// Loads the current projection through the adapter, as a fast-forward of the projection the
    /// session last validated when it holds one, and requires it to be the projection of exactly
    /// the presented history: the binding the session derives from its own graph and branch, the
    /// published control events, the incrementally applied state, and Campaign's own validation.
    ///
    /// # Errors
    ///
    /// Returns the adapter error of the load and `Diverged` when the loaded binding, events,
    /// state, or projection differ.
    pub(crate) fn load(
        &mut self,
    ) -> crate::verdicts::ConformanceResult<campaign_choregraph::ValidatedControlProjection> {
        let expected_binding = self.binding()?;
        let choices = campaign_choregraph::ParentChoices::new();
        let request = campaign_choregraph::ControlProjectionRequest {
            graph: &self.graph,
            branch: &self.branch,
            parent_choices: &choices,
            maximum_events: super::HISTORY_MAX_EVENTS,
            expected_binding: expected_binding.clone(),
            payloads: &self.payloads,
        };
        let previous = self.projection.as_ref();
        let campaign = campaign_load(&request, previous);
        let current = self.harness.adapter.load_history(&request, previous)?;
        if *current.binding() != expected_binding {
            return Err(crate::verdicts::ConformanceError::Diverged {
                expected: "the loaded projection binds the session's own graph and branch",
            });
        }
        let is_published_lineage = current
            .control_events()
            .iter()
            .eq(self.payloads.iter().map(|payload| &payload.control_event));
        if !is_published_lineage {
            return Err(crate::verdicts::ConformanceError::Diverged {
                expected: "the loaded projection carries the published control events",
            });
        }
        if *current.state() != self.state {
            return Err(crate::verdicts::ConformanceError::Diverged {
                expected: "loaded history equals the incrementally applied state",
            });
        }
        if campaign.as_ref() != Ok(&current) {
            return Err(crate::verdicts::ConformanceError::Diverged {
                expected: "the loaded projection equals Campaign's validation of the history",
            });
        }
        self.projection = Some(current.clone());
        debug_assert_eq!(current.control_events().len(), self.payloads.len());
        Ok(current)
    }

    /// The projection one presentation passes: none at cold start, otherwise the projection the
    /// session last loaded.
    ///
    /// # Errors
    ///
    /// Returns `Diverged` for a presentation through a projection the session never loaded.
    pub(crate) fn presented_projection(
        &self,
        presentation: Presentation,
    ) -> crate::verdicts::ConformanceResult<Option<&campaign_choregraph::ValidatedControlProjection>>
    {
        presented(self.projection.as_ref(), presentation)
    }

    /// Presents one history to the adapter's load path and requires the adapter's answer to equal
    /// Campaign's answer for the same request and presentation. The session's held projection is
    /// left unchanged.
    ///
    /// # Errors
    ///
    /// Returns the oracle mismatch of `agree`, and `Diverged` for a presentation through a
    /// projection the session never validated.
    pub(crate) fn present_history(
        &mut self,
        history: &HistoryView,
        presentation: Presentation,
        forbidden: &'static str,
    ) -> crate::verdicts::ConformanceResult<
        campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection>,
    > {
        let expected_binding = campaign_choregraph::derive_projection_binding(
            &history.graph,
            &history.branch,
            self.identities.projection_schema_id,
            self.identities.adapter_id,
            self.identities.control_domain_id,
        )?;
        self.present_bound_history(history, expected_binding, presentation, forbidden)
    }

    /// Presents one history with a caller-chosen expected binding; see `present_history`.
    ///
    /// # Errors
    ///
    /// Returns the oracle mismatch of `agree`, and `Diverged` for a presentation through a
    /// projection the session never validated.
    pub(crate) fn present_bound_history(
        &mut self,
        history: &HistoryView,
        expected_binding: campaign_core::ProjectionBinding,
        presentation: Presentation,
        forbidden: &'static str,
    ) -> crate::verdicts::ConformanceResult<
        campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection>,
    > {
        let previous = presented(self.projection.as_ref(), presentation)?;
        let request =
            hosted::projection_request(history, expected_binding, super::HISTORY_MAX_EVENTS);
        hosted::answer_load(&mut *self.harness.adapter, &request, previous, forbidden)
    }

    /// Presents one cache and one history to the adapter's cache validation, cold or through the
    /// session's held projection, and requires the adapter's answer to equal
    /// `validate_cache_against_history` for the same request.
    ///
    /// # Errors
    ///
    /// Returns the oracle mismatch of `agree`, and `Diverged` for a presentation through a
    /// projection the session never validated.
    pub(crate) fn present_cache(
        &mut self,
        history: &HistoryView,
        cache: &campaign_core::CacheProjection,
        presentation: Presentation,
        forbidden: &'static str,
    ) -> crate::verdicts::ConformanceResult<
        campaign_choregraph::AdapterResult<campaign_core::FrontierState>,
    > {
        let previous = presented(self.projection.as_ref(), presentation)?;
        let request = self.cache_request(history, hosted::Host::holding(previous));
        hosted::answer_cache(&mut *self.harness.adapter, &request, cache, forbidden)
    }
}

/// The projection a presentation passes: none at cold start, otherwise the projection the
/// session last loaded through the adapter.
///
/// # Errors
///
/// Returns `Diverged` for a presentation through a projection the session never loaded.
const fn presented(
    held: Option<&campaign_choregraph::ValidatedControlProjection>,
    presentation: Presentation,
) -> crate::verdicts::ConformanceResult<Option<&campaign_choregraph::ValidatedControlProjection>> {
    match (presentation, held) {
        (Presentation::Cold, _) => Ok(None),
        (Presentation::ThroughHeld, Some(held)) => Ok(Some(held)),
        (Presentation::ThroughHeld, None) => Err(crate::verdicts::ConformanceError::Diverged {
            expected: "the session validated a projection before presenting through it",
        }),
    }
}

/// Campaign's own execution-gate answer: granted exactly when a publication and evidence are
/// presented and Campaign's publication gate accepts them.
pub(crate) fn campaign_gate(
    selection: &campaign_core::SelectionDecision,
    publication: Option<&campaign_choregraph::ControlPublication>,
    evidence: Option<&campaign_choregraph::DurablePublicationEvidence>,
) -> crate::port::ExecutionAuthority {
    let (Some(publication), Some(evidence)) = (publication, evidence) else {
        return crate::port::ExecutionAuthority::Refused;
    };
    if campaign_choregraph::can_execute_selection_after_publication(
        selection,
        publication,
        evidence,
    ) == Ok(true)
    {
        crate::port::ExecutionAuthority::Granted
    } else {
        crate::port::ExecutionAuthority::Refused
    }
}
