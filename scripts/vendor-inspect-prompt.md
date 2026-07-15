Task: a new pristine dsntk version was just vendored (`make vendor-status` shows the version and the pristine base commit; the previous patch layer's commits are in git history under the *previous* pristine base). Audit the upstream changes and re-layer our improvements. Work on a branch; never commit to main.

Follow this procedure — it is the tested workflow from the original vendoring sessions (see docs/performance.md "Measurement infrastructure" and vendor/README.md "Layering model"):

1. AUDIT UPSTREAM. Diff the new pristine tree against the previous pristine tree crate by crate (both are single commits in git history whose messages contain "pristine"). Classify every functional change (ignore pure style/reformatting churn): new features, bug fixes, performance-relevant changes. Check explicitly whether any of our carried patches were adopted upstream — for each PGDMN patch commit, name the upstream file and state adopted / partially adopted / absent, with evidence from the diff, not release notes.

2. RE-LAYER THE PATCH SET. For every commit in the previous patch layer (`git log <prev-pristine>..<prev-layer-tip> -- vendor/`), in the same order: port it onto the new sources as its own commit (original message + "Ported to dsntk <version>."), or record why it is obsolete (adopted upstream / mechanism gone). Ports are re-derivations, not blind cherry-picks — read the old diff, apply the equivalent minimal change, keep `PGDMN:` markers, never reformat surrounding code. Known coupling: H20 must land with H3 (measured interaction); the exhaustive AST walker (H13) must be extended for any new AstNode variants — it fails the build by design.

3. GATE EVERY COMMIT. `make vendor-test` must show only the documented environment-dependent upstream failures (see VENDOR_SKIPS in the Makefile; verify the pristine baseline first so new upstream test names are accounted for). Then the extension suite: `make verify && make test`.

4. RE-MEASURE. `make vendor-bench` before (pristine) and after (re-layered), canary-gated per docs/performance.md methodology (num_add within 12% of its idle baseline; sub-µs cross-build deltas under ~8% are noise). Compare against the numbers in docs/performance.md; investigate anything that regressed beyond the noise floor before accepting it.

5. DEPENDENCY SLICING. Verify the dependency gates still hold: the extension's Cargo.lock must not contain reqwest, rustls, aws-lc-sys, tokio, hyper, or quinn (DEPS-001 slicing). If upstream added new heavy or network-touching dependencies, gate them the same way (off-by-default cargo feature in the vendored crate, upstreamable shape).

6. DOCUMENT. Update vendor/README.md (version), docs/performance.md (numbers and patch inventory), TODO.md (upstreaming status per patch), and BUGHISTORY reoccurrence checks (BUG-003's checklist names vendored code). Commit per repo discipline (/simplify, make verify, bug-check) and push to the PR branch.

Report at the end: upstream-adoption table, per-port notes, measurement deltas, and any patch-layer commits you dropped or changed materially.
