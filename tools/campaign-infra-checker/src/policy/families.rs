//! Identifier families for shared-crate source scans.
//!
//! Families match Rust identifiers split into lowercase words, never comments or strings.

// r[impl campaign.replay_boundary]
const MINIMIZATION_AUTHORITY: crate::policy::scope::Family = crate::policy::scope::Family {
    rule: crate::rules::Rule::REPLAY_BOUNDARY_INVALID,
    name: "minimization authority",
    matcher: crate::policy::scope::Matcher::WordPrefixes(&["minimi"]),
};

const MERGE_AUTHORITY: crate::policy::scope::Family = crate::policy::scope::Family {
    rule: crate::rules::Rule::REPLAY_BOUNDARY_INVALID,
    name: "merge authority",
    matcher: crate::policy::scope::Matcher::Words(&[
        "merge", "merges", "merged", "merging", "merger",
    ]),
};

// r[impl campaign.replay_boundary]
const REPLAY_EXECUTION_AUTHORITY: crate::policy::scope::Family = crate::policy::scope::Family {
    rule: crate::rules::Rule::REPLAY_BOUNDARY_INVALID,
    name: "replay execution or comparison authority",
    matcher: crate::policy::scope::Matcher::WordPair(
        &["replay", "replays"],
        &[
            "execute",
            "executes",
            "execution",
            "executor",
            "run",
            "runs",
            "runner",
            "cursor",
            "cursors",
            "compare",
            "comparison",
            "verdict",
            "truth",
            "drive",
            "driver",
            "start",
        ],
    ),
};

// r[impl campaign.replay_boundary]
const LINEAGE_SELECTION_AUTHORITY: crate::policy::scope::Family = crate::policy::scope::Family {
    rule: crate::rules::Rule::REPLAY_BOUNDARY_INVALID,
    name: "lineage selection authority",
    matcher: crate::policy::scope::Matcher::WordPair(
        &[
            "select",
            "selects",
            "selected",
            "selection",
            "selector",
            "choose",
            "chooses",
            "chosen",
            "pick",
            "picks",
        ],
        &["lineage", "lineages"],
    ),
};

// r[impl campaign.replay_boundary.invalid]
const SEMANTIC_PARENT_AUTHORITY: crate::policy::scope::Family = crate::policy::scope::Family {
    rule: crate::rules::Rule::REPLAY_BOUNDARY_INVALID,
    name: "semantic parent selection authority",
    matcher: crate::policy::scope::Matcher::WordPair(&["semantic"], &["parent", "parents"]),
};

// r[impl campaign.chaoscontrol_migration.seed_runner]
const SEED_RUNNER_RESUME_AUTHORITY: crate::policy::scope::Family = crate::policy::scope::Family {
    rule: crate::rules::Rule::SEED_RUNNER_BOUNDARY,
    name: "seed-runner resume or checkpoint authority",
    matcher: crate::policy::scope::Matcher::Words(&[
        "resume",
        "resumes",
        "resumed",
        "checkpoint",
        "checkpoints",
    ]),
};

// r[impl campaign.chaoscontrol_migration.seed_runner]
const SEED_RUNNER_AGGREGATION_AUTHORITY: crate::policy::scope::Family =
    crate::policy::scope::Family {
        rule: crate::rules::Rule::SEED_RUNNER_BOUNDARY,
        name: "seed-runner aggregation or progress authority",
        matcher: crate::policy::scope::Matcher::Words(&[
            "aggregate",
            "aggregates",
            "aggregation",
            "progress",
        ]),
    };

// r[impl campaign.chaoscontrol_migration.seed_runner]
const SEED_DISPATCH_AUTHORITY: crate::policy::scope::Family = crate::policy::scope::Family {
    rule: crate::rules::Rule::SEED_RUNNER_BOUNDARY,
    name: "multi-seed dispatch authority",
    matcher: crate::policy::scope::Matcher::WordPair(
        &["seed", "seeds"],
        &["dispatch", "runner", "worker", "workers", "progress"],
    ),
};

const UNIVERSAL_WEAVE: crate::policy::scope::Family = crate::policy::scope::Family {
    rule: crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
    name: "Universal Weave identifier",
    matcher: crate::policy::scope::Matcher::Joined(&["weave", "universal"]),
};

pub(crate) const CONFORMANCE: &[crate::policy::scope::Family] = &[
    MINIMIZATION_AUTHORITY,
    MERGE_AUTHORITY,
    REPLAY_EXECUTION_AUTHORITY,
    LINEAGE_SELECTION_AUTHORITY,
    SEMANTIC_PARENT_AUTHORITY,
    SEED_RUNNER_RESUME_AUTHORITY,
    SEED_RUNNER_AGGREGATION_AUTHORITY,
    SEED_DISPATCH_AUTHORITY,
    UNIVERSAL_WEAVE,
];

// r[impl campaign.boundary.history_leak]
// r[impl campaign.consumer_pilots.leak]
// r[impl campaign.findings.semantic_leak]
// r[impl campaign.non_vm_candidate_fixture.foreign_types]
pub(crate) const CORE: &[crate::policy::scope::Family] = &[
    MINIMIZATION_AUTHORITY,
    MERGE_AUTHORITY,
    REPLAY_EXECUTION_AUTHORITY,
    LINEAGE_SELECTION_AUTHORITY,
    SEMANTIC_PARENT_AUTHORITY,
    SEED_RUNNER_RESUME_AUTHORITY,
    SEED_RUNNER_AGGREGATION_AUTHORITY,
    SEED_DISPATCH_AUTHORITY,
    crate::policy::scope::Family {
        rule: crate::rules::Rule::BOUNDARY_HISTORY_LEAK,
        name: "Choregraph history or Universal Weave identifier",
        matcher: crate::policy::scope::Matcher::Joined(&["choregraph", "weave", "universal"]),
    },
    crate::policy::scope::Family {
        rule: crate::rules::Rule::FINDINGS_SEMANTIC_LEAK,
        name: "finding semantic identifier",
        matcher: crate::policy::scope::Matcher::Words(&[
            "defect",
            "defects",
            "bug",
            "bugs",
            "severity",
            "severities",
            "assertion",
            "assertions",
            "crash",
            "crashes",
            "vulnerability",
            "vulnerabilities",
            "proof",
            "proofs",
            "triage",
        ]),
    },
    crate::policy::scope::Family {
        rule: crate::rules::Rule::NON_VM_FOREIGN_TYPES,
        name: "foreign runtime identifier",
        matcher: crate::policy::scope::Matcher::Joined(&[
            "branchcontext",
            "softwareworld",
            "python",
            "pyo3",
        ]),
    },
    crate::policy::scope::Family {
        rule: crate::rules::Rule::NON_VM_FOREIGN_TYPES,
        name: "foreign consumer semantic identifier",
        matcher: crate::policy::scope::Matcher::Words(&[
            "fuse",
            "fuser",
            "prompt",
            "prompts",
            "validator",
            "validators",
        ]),
    },
    crate::policy::scope::Family {
        rule: crate::rules::Rule::CONSUMER_PILOTS_LEAK,
        name: "consumer product or VM identifier",
        matcher: crate::policy::scope::Matcher::Joined(&["chaoscontrol", "animus", "lattice"]),
    },
    crate::policy::scope::Family {
        rule: crate::rules::Rule::CONSUMER_PILOTS_LEAK,
        name: "consumer VM runtime identifier",
        matcher: crate::policy::scope::Matcher::Words(&["kvm", "vmm", "hypervisor"]),
    },
];

// r[impl campaign.boundary.consumer_leak]
pub(crate) const ADAPTER: &[crate::policy::scope::Family] = &[
    MINIMIZATION_AUTHORITY,
    MERGE_AUTHORITY,
    REPLAY_EXECUTION_AUTHORITY,
    LINEAGE_SELECTION_AUTHORITY,
    SEMANTIC_PARENT_AUTHORITY,
    SEED_RUNNER_RESUME_AUTHORITY,
    SEED_RUNNER_AGGREGATION_AUTHORITY,
    SEED_DISPATCH_AUTHORITY,
    UNIVERSAL_WEAVE,
    crate::policy::scope::Family {
        rule: crate::rules::Rule::BOUNDARY_CONSUMER_LEAK,
        name: "consumer product identifier",
        matcher: crate::policy::scope::Matcher::Joined(&["chaoscontrol", "animus", "lattice"]),
    },
];
