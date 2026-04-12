# Training System Operations Guide

> **Purpose:** How the modular training system works — the update workflow, audit cadence, automation, and how to add new content.
>
> **Owner:** Marcus Kidan | **Last Updated:** April 10, 2026

---

## How the System Works

The training content lives in individual markdown files, one per module. Each file has YAML frontmatter with machine-readable metadata (feature dependencies, change sensitivity, status) and a markdown body with the teaching content.

Three things keep it current:

1. **The module files** — Each module owns its content, declares the Postman features it depends on, and defines what product changes should trigger a review.
2. **The Academy Hub** — Reads the module metadata to display status, flag affected modules, and show the overall health of the training content.
3. **Automated monitoring** — A scheduled Cowork task scans Postman's #product-updates Slack channel weekly, compares against each module's `postman_features` list, and produces a review document for human approval.

---

## Directory Structure

```
entries/
  SCHEMA.md              ← Defines the standard format for all entry/module files
  SYSTEM.md              ← This file — operational guide
  meeting-notes.md       ← Auto-discovered notebook (any .md file works)
  standups.md            ← Example: another notebook
  bootcamp/
    M01-welcome.md
    M02-internet-apis.md
    M03-how-apis-work.md
    M04-api-design.md
    M05-security.md
    M06-git-version-control.md
    M07-developer-experience.md
    M08-ai-agents.md
    M09-postman-platform.md
    M10-hands-on-lab.md
    M11-lifecycle-governance.md
    M12-wrap-up.md
  p4p/                   ← Future: P4P 101/201 class files
```

### Entry Files (Notebooks)

Any `.md` file placed directly in `entries/` is automatically discovered by the HTML viewer and displayed as a notebook. No registration or configuration is needed — just drop a file in and reload.

**Rules for entry files:**
- Must start with a date heading: `# MM-DD-YYYY`
- Each subsequent date heading (`# MM-DD-YYYY`) starts a new entry
- File name doesn't matter — it becomes the display name (hyphens/underscores become spaces)
- `SCHEMA.md` and `SYSTEM.md` are excluded automatically

This design keeps individual files small and focused. Split notes however makes sense — by topic, by meeting type, by month, by project. The Library view merges all entries across all files into a single timeline sorted by date.

---

## The Update Workflow

### How Product Changes Flow Through the System

```
Postman ships a change
  → #product-updates Slack channel
    → Cowork weekly scan (review-product-updates skill)
      → Compares against each module's postman_features list
        → Produces a Product Update Review document
          → Human reviews, approves/modifies/skips
            → apply-approved-updates skill writes changes
              → Module file updated (frontmatter status + body content)
              → Hub reflects new state
```

### What Automation Does

1. **Scans #product-updates** — Weekly scheduled task reads the past 7 days of posts
2. **Maps to modules** — Each update is compared against every module's `postman_features` frontmatter field
3. **Classifies impact** — High (direct feature change), Medium (related capability), Low (tangential), None
4. **Produces review document** — Structured markdown with recommended actions for each affected module
5. **Applies approved changes** — After human review, updates module frontmatter (status, dates) and body content as needed

### What Humans Do

1. **Review the Product Update Review** — Check flagged modules and recommended changes for accuracy
2. **Approve, modify, or skip** — Accept automation's suggestions or adjust based on pedagogical judgment
3. **Test in delivery** — Run updated modules in the next session
4. **Capture field notes** — Update facilitator notes in the module file

---

## Audit Cadence

| Change Sensitivity | Audit Frequency | Modules |
|-------------------|----------------|---------|
| 🔴 High | Weekly | M06 (Git), M08 (AI), M09 (Platform), M10 (Hands-On Lab) |
| 🟡 Medium | Bi-weekly | M04 (Design), M05 (Security), M07 (Dev Experience), M11 (Lifecycle) |
| 🟢 Low | Monthly | M01 (Welcome), M02 (Internet & APIs), M03 (How APIs Work), M12 (Wrap-Up) |

---

## Adding a New Module

Whether it's a new bootcamp module or a P4P class, follow these steps:

1. **Choose the right directory** — `bootcamp/` for bootcamp modules, `p4p/` for P4P classes
2. **Follow the schema** — Copy the structure from `SCHEMA.md`. Every field in the frontmatter is required.
3. **Name the file** — Pattern: `{ID}-{slug}.md` (e.g., `M13-advanced-testing.md` or `P4P-101-api-basics.md`)
4. **Declare feature dependencies** — The `postman_features` list is what connects your module to the monitoring system. Be specific. "Collections" is better than "Postman features."
5. **Set change sensitivity** — How often will product changes affect this content? This determines audit frequency.
6. **Write update triggers** — Plain-language descriptions of what kinds of product changes should flag this module for review.
7. **The hub will pick it up** — Once the file exists in the right directory with valid frontmatter, the hub and monitoring skills will include it automatically.

---

## Module Status Lifecycle

```
current → needs_review → update_in_progress → current
                      → not relevant (no change needed) → current
                      → outdated (if unaddressed too long)
```

| Status | Meaning | Action |
|--------|---------|--------|
| `current` | Content matches the current product | None — module is healthy |
| `needs_review` | A product update may affect this module | Review flagged changes, decide if content needs updating |
| `update_in_progress` | Approved changes are being applied | Complete the update, test in delivery, mark current |
| `outdated` | Module has unaddressed changes for 2+ audit cycles | Prioritize for immediate update |

---

## Source Materials

| Document | Purpose |
|----------|---------|
| API Fundamentals Field Guide (api-city-explainer-2.html) | Master metaphor reference — the City Metaphor system |
| V11 Bootcamp Flow ([Google Doc](https://docs.google.com/document/d/1D7o7IYvkrQm2FeCgbh7SisnzF4Xlbc6pA1QJInGh-y4/edit)) | Original facilitator flow |
| Bootcamp Review ([Google Doc](https://docs.google.com/document/d/1D14mLqe-BIz3dseLtvF9IVgUClZmfCICfFERnfStnog/edit)) | Analysis of what needed restructuring |
| Refresh Guide ([Google Doc](https://docs.google.com/document/d/1Uo1oHwETRLwxCRA40Hc1am5toD1TrI3QmUQSuCZ7Y_8/edit)) | Digital Grocery Store hands-on modules |
| Postman V12 Wiki ([Confluence](https://postmanlabs.atlassian.net/wiki/spaces/v12/pages/7131791735/Postman+v12)) | Internal V12 product documentation |

---

## Design Principles

These principles govern how modules are written and maintained:

**Interleaved, not lecture-then-lab.** Every module weaves concept → metaphor → activity → reflection. No "theory block then practice block."

**One metaphor system.** The City Metaphor is the backbone. The internet is a city, APIs are buildings, requests are forms, Postman is the city management platform.

**Each module is self-contained.** A module can be delivered independently without breaking the narrative. Modules 1–12 have a natural flow but aren't hard dependencies.

**Pain points are threaded, not siloed.** Each module surfaces the relevant real-world problem in context rather than collecting all pain points in one place.

**Feature dependencies are explicit.** Every module declares exactly which Postman product features it references. This is the linkage that makes automated monitoring work.
