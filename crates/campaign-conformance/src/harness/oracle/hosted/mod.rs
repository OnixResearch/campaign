//! Presentations under an explicit host: the projection bound the host requests and the
//! validated projection it holds (none at cold start, or any projection it validated earlier).

/// How a host presents one history to the adapter's load and cache paths.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Host<'p> {
    /// Largest control lineage the host's projection request admits.
    pub(crate) maximum_events: u32,
    /// The validated projection the host holds; `None` at cold start.
    pub(crate) previous: Option<&'p campaign_choregraph::ValidatedControlProjection>,
}

impl<'p> Host<'p> {
    /// A host holding `previous` under the session's history bound.
    pub(crate) const fn holding(
        previous: Option<&'p campaign_choregraph::ValidatedControlProjection>,
    ) -> Self {
        Self {
            maximum_events: crate::harness::HISTORY_MAX_EVENTS,
            previous,
        }
    }
}

/// Projection request of one presented history under `expected_binding` and a bound.
pub(super) fn projection_request(
    history: &super::HistoryView,
    expected_binding: campaign_core::ProjectionBinding,
    maximum_events: u32,
) -> campaign_choregraph::ControlProjectionRequest<'_> {
    campaign_choregraph::ControlProjectionRequest {
        graph: &history.graph,
        branch: &history.branch,
        parent_choices: &history.parent_choices,
        maximum_events,
        expected_binding,
        payloads: &history.payloads,
    }
}

/// Asks the adapter's load path and Campaign the same load question and requires equal answers.
///
/// # Errors
///
/// Returns the oracle mismatch of `agree`.
pub(super) fn answer_load(
    adapter: &mut dyn crate::port::ConformanceAdapter,
    request: &campaign_choregraph::ControlProjectionRequest<'_>,
    previous: Option<&campaign_choregraph::ValidatedControlProjection>,
    forbidden: &'static str,
) -> crate::verdicts::ConformanceResult<
    campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection>,
> {
    let campaign = super::campaign_load(request, previous);
    let shell = adapter.load_history(request, previous);
    super::agree(shell, &campaign, forbidden)
}

/// Asks the adapter's cache validation and Campaign the same cache question and requires equal
/// answers.
///
/// # Errors
///
/// Returns the oracle mismatch of `agree`.
pub(super) fn answer_cache(
    adapter: &mut dyn crate::port::ConformanceAdapter,
    request: &campaign_choregraph::CacheValidationRequest<'_>,
    cache: &campaign_core::CacheProjection,
    forbidden: &'static str,
) -> crate::verdicts::ConformanceResult<
    campaign_choregraph::AdapterResult<campaign_core::FrontierState>,
> {
    let campaign = campaign_choregraph::validate_cache_against_history(request, cache);
    let shell = adapter.validate_cache(request, cache);
    super::agree(shell, &campaign, forbidden)
}

impl crate::harness::Session<'_, '_> {
    /// Cache validation request of one presented history under the session's identities.
    pub(super) fn cache_request<'r>(
        &self,
        history: &'r super::HistoryView,
        host: Host<'r>,
    ) -> campaign_choregraph::CacheValidationRequest<'r> {
        campaign_choregraph::CacheValidationRequest {
            graph: &history.graph,
            branch: &history.branch,
            parent_choices: &history.parent_choices,
            maximum_events: host.maximum_events,
            projection_schema_id: self.identities.projection_schema_id,
            adapter_id: self.identities.adapter_id,
            control_domain_id: self.identities.control_domain_id,
            payloads: &history.payloads,
            previous: host.previous,
        }
    }

    /// Presents one history with a caller-chosen expected binding under an explicit host; the
    /// adapter's answer must equal Campaign's. The session's held projection is left unchanged.
    ///
    /// # Errors
    ///
    /// Returns the oracle mismatch of `agree`.
    pub(crate) fn present_hosted_history(
        &mut self,
        history: &super::HistoryView,
        expected_binding: campaign_core::ProjectionBinding,
        host: Host<'_>,
        forbidden: &'static str,
    ) -> crate::verdicts::ConformanceResult<
        campaign_choregraph::AdapterResult<campaign_choregraph::ValidatedControlProjection>,
    > {
        let request = projection_request(history, expected_binding, host.maximum_events);
        answer_load(
            &mut *self.harness.adapter,
            &request,
            host.previous,
            forbidden,
        )
    }

    /// Presents one cache validation request, as a host builds it, to the adapter's cache
    /// validation; the adapter's answer must equal `validate_cache_against_history`.
    ///
    /// # Errors
    ///
    /// Returns the oracle mismatch of `agree`.
    pub(crate) fn present_cache_request(
        &mut self,
        request: &campaign_choregraph::CacheValidationRequest<'_>,
        cache: &campaign_core::CacheProjection,
        forbidden: &'static str,
    ) -> crate::verdicts::ConformanceResult<
        campaign_choregraph::AdapterResult<campaign_core::FrontierState>,
    > {
        answer_cache(&mut *self.harness.adapter, request, cache, forbidden)
    }
}
