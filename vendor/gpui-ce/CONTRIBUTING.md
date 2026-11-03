# Contributing

We welcome contributions from the community and ask that you familiarize yourself with the contents of this document before reporting issues or submitting pull requests!

For this document, `maintainers` are community members who have merge permissions on pull requests within the organization/repository.

All commits, pull requests, and discussions must adhere to the AI_POLICY document.

Much of this document was adapted from existing docs cited in the Sources section.

## Contributors

We use a vouch system for first-time contributors:

1. Open a [discussion in the "Vouch Request"](https://github.com/gpui-ce/gpui-ce/discussions/new?category=vouch-request) category describing what you want to change and why. Follow the template & keep it concise
2. Write in your own voice, don't have an AI write this.
3. A maintainer will comment `!vouch` if approved, which will add you to the vouch list.
4. Once approved, you can submit Pull Requests.

> [!NOTE]
> If you contributed prior to the introduction of the vouch system and wish to continue contributing, you were not automatically added to the list of vouched users. You will need to follow the same process as a first-time contributor to be vouched.

If you aren't vouched, any pull requests you open will be automatically closed. This system exists because open source works on a system of trust, and genAI/LLMs have unfortunately made it so we can no longer trust-by-default because it makes it too trivial to generate plausible-looking but actually low-quality contributions.

If you repeatedly break the rules of this document or repeatedly submit low quality work, you will be denounced; your username is marked as blocked on the vouch list to track bad actors who have wasted our time. All future interactions on this project will be automatically closed. The denouncement list is public, so other projects who trust our maintainer judgement can also block you automatically.

## Quick Guide

### I'd like to contribute

Welcome to the community! All issues are intended to be actionable with clear next steps. You're welcome to pick one and start working on it. For first-time contributors, its recommended to pick something small or a bug to get familiar with the project and community. If you need help or guidance, comment on the issue. Issues that are extra friendly to new contributors are tagged with ["contributor friendly"](https://github.com/gpui-ce/gpui-ce/issues?q=is%3Aissue%20is%3Aopen%20label%3A%22contributor%20friendly%22).

The vouch policy is lovingly adopted from the ghostyy community. Until we have a large sample pool of discussions, please refer to the quality bar of their [vouch requests](https://github.com/ghostty-org/ghostty/discussions/categories/vouch-request) for some examples of the amount of detail in requests.

### Issues are Actionable

The [issue tracker](https://github.com/gpui-ce/gpui-ce/issues) is solely for **actionable items**.

**We not use the issue tracker for discussion or feature requests.** Github Discussions are used to discuss feature requests and other non-actionable topics. Once a discussion reaches a point where a well-understood, actionable item is identified, it is moved to the issue tracker. This pattern makes it easier for maintainers or contributors to find issues to work on since every _issue_ is ready to be worked on.

### I have a bug! / Something isn't working

First, search the issue tracker and discussions for similar issues. Tip: also search for [closed issues](https://github.com/gpui-ce/gpui-ce/issues?q=is%3Aissue%20state%3Aclosed) and [discussions](https://github.com/gpui-ce/gpui-ce/discussions?discussions_q=is%3Aclosed) — your issue might have already been fixed!

> [!NOTE]
> If there is an open issue or discussion that matches your problem, please do not comment on it unless you have valuable insight to add.
>
> GitHub has a very noisy set of default notification settings which sends an email to every participant in an issue/discussion every time someone adds a comment. Instead, use the handy upvote button for discussions, and/or emoji reactions on both discussions and issues, which are a visible yet non-disruptive way to show your support.

If your issue hasn't been reported already, open an ["Issue Triage"](https://github.com/gpui-ce/gpui-ce/discussions/new?category=issue-triage) discussion and make sure to fill in the template **completely**. These reports are vital for maintainers to figure out important details about your setup.

> [!WARNING]
> A _very_ common mistake is to file a bug report either as a Q&A or a Feature Request. **Please don't do this**; its likely maintainers will need to ask for information that is covered by the Issue Triage form, which wastes time and energy for everyone.
>
> Because of this, please make sure that you only use the "Issue Triage" category for reporting bugs — thank you!

### I have an idea for a feature

Like bug reports, first search through both issues and discussions and try to find if your feature has already been requested. Otherwise, open a discussion in the ["Feature Requests, Ideas"](https://github.com/gpui-ce/gpui-ce/discussions/new?category=feature-requests-ideas) category.

### I've implemented a feature

Pull requests should be associated with a previously accepted issue. **If you open a pull request for something that wasn't previously discussed**, it may be closed or remain stale for an indefinite period of time. Pull Requests without context or planning place a large amount of work on maintainers with limited bandwidth, therefore blind PRs are a lower priority.

If you open a pull request without discussion first, you may find that the community already has discussed this and found a better approach, or that it already was deemed not to be a fit for the project. Since many ideas have come up before - but they haven't been implemented because they might be deceptively nuanced, or there may be strong opinions around them - bringing it up to the community first helps discover past discussions, so your design can be informed by them.

Issues tagged with ["feature"](https://github.com/gpui-ce/gpui-ce/issues?q=is%3Aissue+state%3Aopen+type%3AFeature) represent accepted, well-scoped feature requests. If you implement an issue tagged with feature as described in the issue, your pull request will have a higher degree of priority during review since a plan has already been accepted by the community.

If there is no issue representing your changes, open a discussion and a link to your branch (I have an idea for a feature, cited above).

> [!NOTE]
> **Pull requests are NOT a place to discuss feature design.** Please do not open a WIP pull request to discuss a feature. Instead, use a discussion and link to your branch.

It is strongly recommended that authors contribute smaller patches first, to build up communal trust and help you understand the project and the community around it works.

## Authoring Guidelines

### Commits

The maintainer team cares more about the contents of commits than the total sum of contents in a pull request. We review each commit separately, and we don't squash-merge the PR (so please manually squash any fixup commits before sending for review). Additionally, Github Stacked Pull Requests can only be created from branches in the target repository, so keeping clean commits is the closest approach we have for stacked changes.

Each commit should ideally do one thing. For example, if you need to refactor a function in order to add a new feature cleanly, put the refactoring in one commit and the new feature in a different commit. If the refactoring itself consists of many parts, try to separate out those into separate commits. Include tests and documentation in the same commit as the code they test and document. Ideally changes which add features or apis should contain additions of or changes to existing [examples](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#examples) in the project (of course these examples must still compile and be testable on all commits).

The commit message should describe the changes in the commit. Please read [How to Write a Git Commit Message](https://cbea.ms/git-commit/) and use a similar format when writing your commit descriptions, and explain the reason for the change unless its obvious (such obvious cases would be how error, None, and panics are handled). If the commit closes an open issue or feature request for the project, include the `Closes #<issue-number>` (for features) or `Fixes #<issue-number>` (for bugs) in the footer description. If the commit implements an RFC, it should include `Implements #<issue-number>` in the footer description.

All commits should update the CHANGELOG according to that document's specifications.

> [!NOTE]
> Its recommended to try using [jujutsu (jj)](https://docs.jj-vcs.dev/latest/) as it makes managing commits (and squashing/rebasing/conflicts) significantly easier. [Steve Klabnik's guide/book](https://steveklabnik.github.io/jujutsu-tutorial/introduction/introduction.html) is commonly referred to.

### Code Reviews

All submissions, including submissions by project members/maintainers, require review. We use GitHub pull requests for this purpose.

As is common in open-source, all maintainers are volunteers and the project is limited on both maintainers and maintainer bandwidth. To maximize the chances of a pull-request being timely reviewed and approved, do what you can to provide reviewers (and other future readers) with the necessary context.

Community members that are not maintainers are strongly encouraged to review and comment on each other's pull requests at-will/ad-hoc. Each reviewer should leave a "marked" review, using the github review feature (which allows individuals to identify whether they approve, request changes, or are just leaving comments). Maintainers are more likely to prioritize pull requests that have been reviewed by the community over ones that have not. Community members who are familiar with the project and each other are welcome to `@mention` other members with subject-matter-expertise, or ask in discord who has experience with system domains and can review.

You can affect the speed of review by:

- following documented policies (this document and AI_POLICY)
- If adding a new feature; explain why the feature is useful, how it works from the user's point of view, how it's designed, and what the limitations of your design are. If you have considered other designs, it may be useful to mention them.
  - add [examples](https://github.com/gpui-ce/gpui-ce/tree/main/crates/gpui/examples) showcasing how the feature is leveraged by projects using gpui
- review other's pull requests, which reduces the load on maintainers and helps you ramp onto the project
- ask non-maintainer community members to review your change
- ensure CI is passing

When you address comments on a PR, don't make the changes in a commit on top (as is typical on GitHub). Instead, please make the changes in the appropriate commit. You can do that by creating a new commit on top of the initial commit and then squash in the changes when you're done.

Mark conversation threads as resolved when you have addressed them. If you realize that a comment requires non-trivial changes, write a reply instead and ask your reviewer to take another look after updating the code.

All Pull Requests are expected to pass all CI checks. If any CI checks are failing, PR authors are expected to resolve any issues. If trying to understand why CI is failing takes longer than an hour, authors are expected to leave a comment on the PR identifying the CI failure and what about the reports are confusing (so maintainers and community members can clarify & work to improve CI reports).

If your pull request meets all expectations and is still idle after 2 weeks, feel free to bump the PR comments (if someone has reviewed it already) or mention it in discord. We as a community understand that we are completely decentralized and each have a very limited bandwidth, as long as everyone is respectful there will be no hard feelings about bumps when there have been no discussions/communication.

## Learning Rust

In addition to the [Rust Book](https://doc.rust-lang.org/book/) and the other excellent resources at https://www.rust-lang.org/learn, we recommend the [Comprehensive Rust](https://google.github.io/comprehensive-rust/) mini-course for an overview, especially if you are familiar with C++.

## Sources/Inspirations

- [ghostty](https://github.com/ghostty-org/ghostty/blob/5dc28bb8eebaf57a6c793a406bfea8c632d4fa94/CONTRIBUTING.md)
- [jujutsu vcs](https://docs.jj-vcs.dev/latest/contributing/)
- [bevy](https://bevy.org/learn/contribute/helping-out/opening-pull-requests/)
