# Generative AI/LLM Policy

Many other projects have written very well articulated understandings of the state of open-source and genAI/LLMs, so this document will not attempt to rearticulate the same points, especially due to limited maintainer bandwidth. Source are linked at the bottom of this document, but please read [Bevy](https://bevy.org/learn/contribute/policies/ai/), [Godot](https://godotengine.org/article/contribution-policy-2026/), and [Rust-lang](https://blog.rust-lang.org/inside-rust/2026/08/05/rust-langrust-is-adopting-an-llm-policy/) articles for a basic rundown of the environment and struggles of open-source projects with a small maintainer team.

Much of this text has been adapted from the source documents, as is the way of open-source.

## Policy

**Repository Markup**

The gpui-ce repositories will not contain markdown files instructing LLMs on how to process the project.

**No LLM-generated prose or media.**

When our maintainers volunteer their time to review your issue, PR, or proposal, they do not want to talk to a machine. This is a basic principle of respect. The same goes for outward-facing prose (e.g. documentation).

genAI/LLMs must not be used to write public-facing prose, including; documentation, changelog updates, issues, Pull-Request descriptions, or release notes. genAI/LLMs must not be used to generate illustrations, art, or other images nor audio, video, etc.

We want to talk to _you_, not a machine. Do not paste LLM-generated output directly into conversations with humans, no matter where they take place. genAI/LLMs may be used to assist for translation and accessibility purposes, but please include the original untranslated text as well. This includes, but is not limited to:

- Creating pull requests and responding to feedback
- Creating or replying to repository (e.g. Github) issues
- Posting messages in social community areas (e.g. Discord and GitHub Discussions)

**No genAI/LLM-authored commits.** Only humans can be commit authors.

**All genAI/LLMs usage in any form must be disclosed.**

genAI/LLMs must not be used to generate a substantial amount of changes.

Any commits that include genAI/LLM assisted changes, or Pull Requests that include such commits, must clearly identify that the change was LLM assisted and the extent of the work that was assisted by genAI/LLMs.

If you use genAI/LLMs in discovering bugs/issues and report such issues to the issue tracker, you must clearly identify that an LLM was used to do so.

**Quality.**

The policy holds LLM-generated changes to a _higher_ bar than human-authored changes, not a lower one. All genAI/LLM assisted commits:

1. must have tests covering all changes, full stop, regardless of how hard it is to do so.
2. must not generate soundness-critical changes unless the author is already a domain expert, and even then it's strongly discouraged.

It is recommended that any genAI/LLM assisted changes be isolated into their own commits, especially when contained in Pull Requests via stacked commits.

It is expected that Pull Requests containing genAI/LLM assisted changes will take longer to review. Such changes will require more reviews by the community and will require subject matter experts from affected domains to approve the changes. Authors should expect such changes to take longer to be approved and likely require more revisions/involvement by the author.

**Ownership.**

As a contributor, you are responsible for everything you submit. You must personally understand every line of code or documentation that you put forward for review, and be able to articulate the design rationale and implementation tradeoffs when asked. Work within your skill level and knowledge of the project.

If you can't explain what your changes do and how they interact with the greater system without the aid of genAI/LLM tools, do not contribute to this project. If you would like help understanding code that is already part of the project, please ask in social community channels (e.g. Discord or GitHub Discussions).

**Reviewer Refusal.**

Contributors, maintainers, and community members are not required to review LLM-assisted work, use genAI/LLM tools, or engage in discussions about genAI/LLMs. We want to make sure that those who reject genAI/LLMs remain a valued part of both the work and community of gpui-ce.

Individual reviewers are empowered to request changes to contributions that fail to live up to these norms, or in cases where the work is not salvageable (commonly due to a lack of understanding by the original author), close them completely.

**Bad drivers will be blocked.**

People who violate this policy (e.g. by providing bad contributions that are clearly LLM-generated or slop) will loose our trust and be revoked from the vouch list (see CONTRIBUTING). This will cause all future contributions to be blocked and automatically closed.

We deeply value helping junior developers or peers new to the project learn and grow; if you're interested in aiding the project, then don't use genAI/LLMs in ways that violate this policy.

## There are Humans Here

Please remember that gpui-ce is maintained by humans.

Every discussion, issue, and pull request is read and reviewed by humans. It is a boundary point at which people interact with each other and collaborate to accomplish our goals. It is rude and disrespectful to approach this boundary with low-effort, unqualified work, since it puts the burden of validation on the maintainer.

Please do not use AI and agents to communicate with the community, including Discord, issues, and pull requests.

## Sources for further reading

### Context on Software Ecosystem and Outcomes

- [fallible: It Breaks a Village](https://blog.fallible.net/it-breaks-a-village/)
- [Code is a wedge and programmers are tools](https://lia-sae.net/articles/2026/02/code-is-a-wedge.html)

### AI Policies

- [Rust LLM Policy](https://blog.rust-lang.org/inside-rust/2026/08/05/rust-langrust-is-adopting-an-llm-policy/)
- [Bevy AI Policy](https://bevy.org/learn/contribute/policies/ai/)
- [Godot Contribution Policy](https://godotengine.org/article/contribution-policy-2026/)
- [Jujutsu AI Policy](https://github.com/jj-vcs/jj/blob/ab0747494875930460239f62155d50be768a9452/AI_POLICY.md)
- [Ghostty AI Policy](https://github.com/ghostty-org/ghostty/blob/5dc28bb8eebaf57a6c793a406bfea8c632d4fa94/AI_POLICY.md)
