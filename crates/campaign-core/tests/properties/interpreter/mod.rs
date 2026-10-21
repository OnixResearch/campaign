//! Interprets generated scripts into valid control histories through public APIs only.
//!
//! A scripted call that a declared bound denies is recorded as a typed rejection and leaves the
//! history unchanged; any other denial fails the property because the script is always exact.

const ADMIT_DENIALS: [campaign_core::CampaignError; 4] = [
    campaign_core::CampaignError::BoundExceeded,
    campaign_core::CampaignError::BudgetExhausted,
    campaign_core::CampaignError::CandidateOverDepth,
    campaign_core::CampaignError::ControlEventLimitReached,
];
const GUIDE_DENIALS: [campaign_core::CampaignError; 3] = [
    campaign_core::CampaignError::BoundExceeded,
    campaign_core::CampaignError::InvalidGuidanceShape,
    campaign_core::CampaignError::ControlEventLimitReached,
];
const EVENT_DENIALS: [campaign_core::CampaignError; 1] =
    [campaign_core::CampaignError::ControlEventLimitReached];

/// One planner call: its input state, exact request, and returned plan.
pub(crate) struct Planning {
    pub(crate) state: campaign_core::FrontierState,
    pub(crate) request: campaign_core::SelectionRequest,
    pub(crate) plan: campaign_core::DecisionPlan,
    /// True when the planned selection was published as the next control event.
    pub(crate) published: bool,
}

/// One bound denial and the state the denied call received.
pub(crate) struct Rejection {
    pub(crate) state: campaign_core::FrontierState,
    pub(crate) error: campaign_core::CampaignError,
}

/// A valid control history with the state reconstructed after every accepted event.
pub(crate) struct History {
    pub(crate) events: Vec<campaign_core::CampaignControlEvent>,
    /// `checkpoints[k]` is the state after `events[..=k]`.
    pub(crate) checkpoints: Vec<campaign_core::FrontierState>,
    pub(crate) plannings: Vec<Planning>,
    pub(crate) rejections: Vec<Rejection>,
    /// Published selections in publication order.
    pub(crate) selections: Vec<campaign_core::SelectionDecision>,
    next_label: u64,
}

impl History {
    pub(crate) fn state(&self) -> Result<&campaign_core::FrontierState, String> {
        self.checkpoints
            .last()
            .ok_or_else(|| String::from("a history always holds its admission state"))
    }

    fn admitted(script: &crate::suite::strategies::Script) -> Result<Self, String> {
        let projection = crate::suite::fixture::projection()?;
        let roots = (0..script.roots)
            .map(|index| crate::suite::fixture::root(&projection, index))
            .collect();
        let budgets = (0_u64..)
            .zip(&script.budgets)
            .map(|(index, amount)| campaign_core::ResourceAmount {
                dimension_id: crate::suite::fixture::dimension(index),
                amount: *amount,
            })
            .collect();
        let input = crate::suite::fixture::admission(
            crate::suite::fixture::bounds(&script.limits)?,
            roots,
            budgets,
            vec![crate::suite::fixture::schema(0)],
        )?;
        let (event, state) = crate::suite::fixture::campaign(input)?;
        Ok(Self {
            events: vec![event],
            checkpoints: vec![state],
            plannings: Vec::new(),
            rejections: Vec::new(),
            selections: Vec::new(),
            next_label: 0,
        })
    }

    pub(crate) fn accept(
        mut self,
        outcome: campaign_core::CampaignResult<campaign_core::CampaignControlEvent>,
        tolerated: &[campaign_core::CampaignError],
    ) -> Result<Self, String> {
        let event = match outcome {
            Ok(event) => event,
            Err(error) => return self.reject(error, tolerated),
        };
        self.events.push(event);
        match campaign_core::reconstruct(&self.events) {
            Ok(next) => {
                self.checkpoints.push(next);
                assert_eq!(self.checkpoints.len(), self.events.len());
                Ok(self)
            }
            Err(error) => {
                self.events.pop();
                self.reject(error, tolerated)
            }
        }
    }

    fn reject(
        mut self,
        error: campaign_core::CampaignError,
        tolerated: &[campaign_core::CampaignError],
    ) -> Result<Self, String> {
        if !tolerated.contains(&error) {
            return Err(format!(
                "scripted call denied outside its bounds: {error:?}"
            ));
        }
        let state = self.state()?.clone();
        self.rejections.push(Rejection { state, error });
        Ok(self)
    }

    fn admit(
        mut self,
        drafts: &[crate::suite::strategies::Draft],
        resubmit: Option<u8>,
    ) -> Result<Self, String> {
        let state = self.state()?.clone();
        let mut records = Vec::with_capacity(drafts.len());
        for draft in drafts {
            let parent = *pick(&state.known_moments, draft.parent_pick)?;
            records.push(crate::suite::fixture::record(
                &state,
                self.next_label,
                parent,
                crate::suite::fixture::costs(&draft.costs),
            ));
            self.next_label = self.next_label.saturating_add(1);
        }
        let duplicate = resubmit.and_then(|index| pick(&state.candidates, index).ok());
        if let Some(existing) = duplicate {
            records.push(existing.candidate.clone());
        }
        let outcome = campaign_core::admit_candidates(
            &state,
            records,
            crate::suite::fixture::rule(duplicate.is_some()),
            crate::suite::fixture::coalescing_policy(),
        );
        self.accept(outcome, &ADMIT_DENIALS)
    }

    fn guide(self, step: &crate::suite::strategies::GuideStep) -> Result<Self, String> {
        let crate::suite::strategies::GuideStep {
            pick: index,
            ranks,
            lift,
        } = step;
        let lift = *lift;
        let state = self.state()?;
        let Ok(candidate) = pick(&state.candidates, *index) else {
            return Ok(self);
        };
        let candidate_id = candidate.candidate.candidate_id;
        let record = match &candidate.latest_guidance {
            None => crate::suite::fixture::guidance(
                state,
                candidate_id,
                crate::suite::fixture::FIRST_GENERATION,
                ranks.clone(),
            ),
            Some(latest) if lift == 0 => crate::suite::fixture::guidance(
                state,
                candidate_id,
                latest.generation,
                latest.ranks.clone(),
            ),
            Some(latest) => {
                let mut lifted = latest.ranks.clone();
                let first = lifted
                    .first_mut()
                    .ok_or_else(|| String::from("admitted guidance has a first rank"))?;
                *first = first.saturating_add(i64::from(lift));
                crate::suite::fixture::guidance(
                    state,
                    candidate_id,
                    latest.generation.saturating_add(1),
                    lifted,
                )
            }
        };
        let outcome = campaign_core::admit_guidance(state, record);
        self.accept(outcome, &GUIDE_DENIALS)
    }

    fn select(
        mut self,
        exploration: &crate::suite::strategies::Exploration,
        draws: Option<crate::suite::requests::Draws>,
        counter: u64,
    ) -> Result<Self, String> {
        let state = self.state()?.clone();
        let request = match draws {
            None => crate::suite::requests::request(
                &state,
                crate::suite::requests::profile(campaign_core::SelectionMode::Ranked)?,
                None,
            ),
            Some(draws) => {
                let profile = crate::suite::requests::exploratory(
                    &state,
                    exploration.threshold,
                    exploration.order.clone(),
                )?;
                let ticket = crate::suite::requests::ticket(&state, &profile, draws, counter)?;
                crate::suite::requests::request(&state, profile, Some(ticket))
            }
        };
        let plan =
            crate::suite::fixture::observed(crate::suite::requests::plan(&state, request.clone()))?;
        let decision = plan.decision.clone();
        let policy_inputs = (request.profile.clone(), request.entropy.clone());
        self.plannings.push(Planning {
            state,
            request,
            plan,
            published: false,
        });
        let campaign_core::NextDecision::Select(selection) = decision else {
            return Ok(self);
        };
        let published = self.checkpoints.len();
        let (profile, entropy) = policy_inputs;
        let event = campaign_core::SelectionEvent::new((*selection).clone(), profile, entropy)
            .map(campaign_core::CampaignControlEvent::SelectionPublished);
        self = self.accept(event, &EVENT_DENIALS)?;
        if self.checkpoints.len() > published {
            self.selections.push(*selection);
            if let Some(planning) = self.plannings.last_mut() {
                planning.published = true;
            }
        }
        Ok(self)
    }

    fn settle(self, index: u8) -> Result<Self, String> {
        let state = self.state()?;
        let Ok(outstanding) = pick(&state.outstanding, index) else {
            return Ok(self);
        };
        let observation = crate::suite::results::result_for(state, outstanding)?;
        let outcome = campaign_core::admit_result(state, observation);
        self.accept(outcome, &EVENT_DENIALS)
    }
}

fn pick<T>(items: &[T], index: u8) -> Result<&T, String> {
    items
        .get(crate::suite::fixture::position(index, items.len())?)
        .ok_or_else(|| String::from("picked index is within the collection"))
}

/// Interprets every scripted step in order.
pub(crate) fn run(script: &crate::suite::strategies::Script) -> Result<History, String> {
    let mut history = History::admitted(script)?;
    for (counter, step) in (0_u64..).zip(&script.steps) {
        history = match step {
            crate::suite::strategies::Step::Admit { drafts, resubmit } => {
                history.admit(drafts, *resubmit)?
            }
            crate::suite::strategies::Step::Guide(step) => history.guide(step)?,
            crate::suite::strategies::Step::Select { draws } => {
                history.select(&script.exploration, *draws, counter)?
            }
            crate::suite::strategies::Step::Settle { pick } => history.settle(*pick)?,
        };
    }
    assert_eq!(history.checkpoints.len(), history.events.len());
    Ok(history)
}
