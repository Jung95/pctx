# PCTX01 control admission independent review

Scope: PCTX01-G03-A07–A11; specification §14 and §38. Read-only review by
/root/work_control; reviewer did not edit sources or run competing builds.
The quota helper was implemented by /root/requirements and integrated by the
parent; that implementation handback is not an independent quota review.

The review found a blocking regression: private schedule::plan is also called
by exact_plan during Install/Uninstall. Removing its existing provider guard
made malformed reviewed JSON reach unreachable!() and exit101. The actual
before-fix native failure is retained in control-admission-install-initial.log.
The parent restored a shared schedule_plan_arguments call in both frontend
validation and private plan. The native reviewed-install regression now requires
INVALID_ARGUMENT2 and unchanged files; no native installation occurs.

The reviewer also requested valid-grammar/non-owner evidence. Five actual CLI
routes retain POLICY_DENIED5 and exact state snapshots. Final scoped read-only
review found no remaining blocker in this repair. This does not qualify the
entire official task, all command routes, or native Linux/Windows.
