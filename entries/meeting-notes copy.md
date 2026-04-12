# 04-10-2026

## Sprint Planning

- Reviewed backlog priorities for Q2
- <span data-tag="decision">**Focus areas:** API gateway migration, dashboard redesign</span>
- <span data-tag="test"><span data-tag="test-2">Assigned story points to top 8 items</span></span>
- Marcus to draft technical RFC for gateway approach by Monday

### Action Items
- Sarah: finalize design mocks for settings page
- Dev team: spike on WebSocket support (2 days)
- Marcus: sync with platform team on auth token changes

---

> <span data-tag="test">"Ship the gateway MVP before expanding scope." — consensus from the group</span>

# 04-07-2026

## Design Review

- Walked through new onboarding flow mockups
- Feedback: simplify step 3, too many form fields
- Dark mode contrast issues flagged on the card components
- Agreed to drop the tutorial modal in favor of inline hints

### Decisions
- <span data-tag="decision">Go with **Option B** for the navigation layout</span>
- <span data-tag="decision">Keep sidebar collapsible on mobile</span>
- <span data-tag="decision">Push color system update to next sprint</span>

# 04-03-2026

## Standup / Check-in

- API versioning PR merged — v2 endpoints live in staging
- CI pipeline flaky on integration tests, investigating
- <span data-tag="blocker">**Blocker:** waiting on security review for OAuth scopes</span>

### Notes
- Reminder: company all-hands Friday at 2pm
- New team member (Jordan) starting next Monday
- Need to update the dev environment setup docs before then

# 03-28-2026

## Retro

### What went well
- Shipped auth flow ahead of schedule
- Cross-team collaboration with design was smooth
- Test coverage improved from 64% to 78%

### What could improve
- Too many context switches mid-sprint
- Stand-up running long — try 10-minute timebox
- Deploy process still requires manual steps

### Action Items
- <span data-tag="action-item">Automate staging deploys via GitHub Actions</span>
- <span data-tag="action-item">Marcus: set up a shared "decisions log" doc</span>
- <span data-tag="action-item">Trial async standups for one sprint</span>

# 03-21-2026

## Architecture Discussion

- Evaluated three approaches for event-driven messaging:
  1. **RabbitMQ** — mature, good for task queues
  2. **Kafka** — better for high-throughput streaming
  3. **Redis Streams** — lightweight, already in our stack
- <span data-tag="decision">Leaning toward Redis Streams for MVP, Kafka for scale</span>
- Need to benchmark latency under load before deciding

### Follow-up
- Dev to set up a comparison test environment
- Review results at next week's meeting
- Document trade-offs in the ADR repo
