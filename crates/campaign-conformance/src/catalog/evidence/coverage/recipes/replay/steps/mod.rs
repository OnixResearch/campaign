//! One recipe step through the adapter.

/// Handle of the first child moment a recipe result reports; later ones follow it.
const FIRST_CHILD: u32 = 21;

/// Findings a finding-reporting result carries.
const REPORTED_FINDINGS: u32 = 1;

/// Replay trail: plan receipts made so far, the history before the latest publication (with
/// Campaign's projection of it and its state), the state before every publication so far, the
/// latest state before any stop, and the child moments admitted so far.
pub(super) struct Trail {
    pub(super) plans: alloc::vec::Vec<(u32, campaign_core::CampaignReceipt)>,
    pub(super) before: Option<crate::catalog::evidence::lineages::forks::Base>,
    pub(super) unstopped: campaign_core::FrontierState,
    pub(super) prefixes: alloc::vec::Vec<campaign_core::FrontierState>,
    pub(super) children: u32,
}

/// The recipe operation at `position` in suite order.
fn operation(
    position: usize,
) -> crate::verdicts::ConformanceResult<crate::vocabulary::OperationHandle> {
    super::OPERATIONS
        .get(position)
        .copied()
        .ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the recipe names one of its operations",
        })
}

/// One recipe operation below the root at unit cost.
fn scenario(
    position: usize,
) -> crate::verdicts::ConformanceResult<crate::vocabulary::ScenarioOperation> {
    Ok(crate::catalog::common::operation(
        operation(position)?,
        crate::harness::history::ROOT,
        crate::catalog::common::UNIT_COST,
    ))
}

impl Trail {
    /// Holds Campaign's projection of the current history and captures it as the history before
    /// the next publication.
    fn mark(
        &mut self,
        session: &mut crate::harness::Session<'_, '_>,
    ) -> crate::verdicts::ConformanceResult<()> {
        session.hold_current()?;
        let view = session
            .presented_projection(crate::harness::oracle::Presentation::ThroughHeld)?
            .cloned()
            .ok_or(crate::verdicts::ConformanceError::Diverged {
                expected: "the replay holds the projection of its current history",
            })?;
        self.prefixes.push(session.state.clone());
        self.before = Some(crate::catalog::evidence::lineages::forks::Base {
            history: session.history(),
            state: view.state().clone(),
            view,
        });
        Ok(())
    }

    /// Replays one step.
    ///
    /// # Errors
    ///
    /// Returns the adapter, Campaign, or Choregraph error of the step, and `Diverged` when it
    /// finds nothing to act on.
    pub(super) fn step(
        &mut self,
        session: &mut crate::harness::Session<'_, '_>,
        step: super::super::Step,
    ) -> crate::verdicts::ConformanceResult<()> {
        if let super::super::Step::Select {
            is_exploratory,
            settlement,
        } = step
        {
            return self.select(session, is_exploratory, settlement);
        }
        if let super::super::Step::Stop(status) = step {
            return self.stop(session, status);
        }
        if let super::super::Step::Retire { is_policy } = step {
            let retained = removals::retained(session)?.ok_or(
                crate::verdicts::ConformanceError::Diverged {
                    expected: "the recipe retires a retained operation",
                },
            )?;
            self.mark(session)?;
            return removals::retire(session, retained, is_policy);
        }
        if step == super::super::Step::RetireRemaining {
            while let Some(retained) = removals::retained(session)? {
                self.mark(session)?;
                removals::retire(session, retained, false)?;
            }
            return Ok(());
        }
        self.mark(session)?;
        if let super::super::Step::Prune(pruned) = step {
            return removals::prune(session, pruned);
        }
        if step == super::super::Step::Batch {
            session.admit(&[scenario(0)?, scenario(1)?, scenario(2)?])?;
            return Ok(());
        }
        if step == super::super::Step::CoalescedBatch {
            session.admit(&[scenario(3)?, scenario(0)?])?;
            return Ok(());
        }
        guidance::publish(session, step)
    }

    /// Plans, publishes, and settles one selection.
    fn select(
        &mut self,
        session: &mut crate::harness::Session<'_, '_>,
        is_exploratory: bool,
        settlement: super::super::Settlement,
    ) -> crate::verdicts::ConformanceResult<()> {
        let draw = is_exploratory.then_some(crate::catalog::evidence::claims::EXPLORE);
        let planned = session.plan(draw)?;
        let kind = if is_exploratory {
            super::super::super::kinds::EXPLORATORY_PLAN
        } else {
            super::super::super::kinds::RANKED_PLAN
        };
        self.plans.push((
            kind,
            crate::catalog::evidence::claims::plan_receipt(session, &planned)?,
        ));
        self.mark(session)?;
        let selection = session.execute_planned(&planned)?;
        if settlement == super::super::Settlement::Outstanding {
            return Ok(());
        }
        if settlement == super::super::Settlement::Closure {
            self.mark(session)?;
            return session.close(&selection);
        }
        let has_child = matches!(
            settlement,
            super::super::Settlement::Child | super::super::Settlement::Both
        );
        let findings = if matches!(
            settlement,
            super::super::Settlement::Finding | super::super::Settlement::Both
        ) {
            REPORTED_FINDINGS
        } else {
            0
        };
        let mut reported = alloc::vec::Vec::with_capacity(1);
        if has_child {
            let handle = crate::vocabulary::MomentHandle(FIRST_CHILD.saturating_add(self.children));
            self.children = self.children.saturating_add(1);
            session.add_moment(crate::catalog::common::child(handle))?;
            reported.push(handle);
        }
        self.mark(session)?;
        session.settle(
            &selection,
            &crate::vocabulary::ScenarioOutcome {
                children: reported,
                findings,
            },
        )?;
        Ok(())
    }

    /// Records the class's stop: a planner stop planned for a ranked and an exploratory request
    /// (both plan receipts kept), a policy stop under the finding-limit reason, the host-capacity
    /// consumer stop, or the adapter's consumer stop.
    fn stop(
        &mut self,
        session: &mut crate::harness::Session<'_, '_>,
        status: super::super::super::features::Status,
    ) -> crate::verdicts::ConformanceResult<()> {
        let reason = match status {
            super::super::super::features::Status::Policy => {
                campaign_core::StopReason::PolicyRequested {
                    reason_id: campaign_core::StoppingRule {
                        finding_limit: Some(1),
                    }
                    .policy_reason_id()?
                    .ok_or(campaign_core::CampaignError::InvalidConfig)?,
                }
            }
            super::super::super::features::Status::HostCapacity => {
                campaign_core::StopReason::ConsumerRequested {
                    reason_id: campaign_choregraph::control_capacity_reason_id(),
                }
            }
            super::super::super::features::Status::Consumer
            | super::super::super::features::Status::Running => session.consumer_stop()?,
            _ => self.planned_stop(session, |planned| planned == status)?,
        };
        self.mark(session)?;
        session.stop(reason)?;
        Ok(())
    }

    /// Plans the stop of the bare admission (the frontier stop, or the control-event stop under a
    /// control-event bound that admits no selection) for a ranked and an exploratory request and
    /// keeps both plan receipts (the host records other events instead), so every lineage holds a
    /// planner stop's plan receipt before it was recorded.
    ///
    /// # Errors
    ///
    /// Returns the adapter or Campaign error of a plan and `Diverged` when the planner does not
    /// stop.
    pub(super) fn admission_stop(
        &mut self,
        session: &mut crate::harness::Session<'_, '_>,
    ) -> crate::verdicts::ConformanceResult<()> {
        debug_assert!(
            session.state.candidates.is_empty(),
            "the bare admission holds no candidate"
        );
        let reason =
            self.planned_stop(session, super::super::super::features::Status::is_planned)?;
        debug_assert!(!matches!(
            reason,
            campaign_core::StopReason::ConsumerRequested { .. }
                | campaign_core::StopReason::PolicyRequested { .. }
        ));
        Ok(())
    }

    /// The planner stop of a status `is_status` accepts for a ranked and an exploratory request;
    /// both plan receipts are kept.
    fn planned_stop(
        &mut self,
        session: &mut crate::harness::Session<'_, '_>,
        is_status: impl Fn(super::super::super::features::Status) -> bool,
    ) -> crate::verdicts::ConformanceResult<campaign_core::StopReason> {
        let mut reason = None;
        for draw in [None, Some(crate::catalog::evidence::claims::EXPLORE)] {
            let planned = session.plan(draw)?;
            let campaign_core::NextDecision::Stop(stop) = &planned.plan.decision else {
                return Err(crate::verdicts::ConformanceError::Diverged {
                    expected: "the recipe's planner stops for its class",
                });
            };
            let mut stopped = session.state.clone();
            stopped.stopped = Some(stop.as_ref().clone());
            crate::catalog::common::require(
                is_status(super::super::super::features::of(&stopped).status),
                "the recipe's planner stops for its class",
            )?;
            reason = Some(stop.reason.clone());
            self.plans.push((
                super::super::super::kinds::UNRECORDED_STOP_PLAN,
                crate::catalog::evidence::claims::plan_receipt(session, &planned)?,
            ));
        }
        reason.ok_or(crate::verdicts::ConformanceError::Diverged {
            expected: "the recipe's planner stops for its class",
        })
    }
}

mod guidance;
mod removals;
