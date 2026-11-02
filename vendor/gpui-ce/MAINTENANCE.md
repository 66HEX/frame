# Maintenance

Misc info for maintainers on complex project setup/automation details.

## Vouch

[gpui-ce vouch](https://github.com/gpui-ce/vouch) forked from [ghostty vouch](https://github.com/mitchellh/vouch)

Provides github actions for automating processing of issues/PRs against vouched users. Requires a [github app](https://docs.github.com/apps/creating-github-apps/setting-up-a-github-app/creating-a-github-app) to [acquire a token](https://github.com/actions/create-github-app-token) in order to apply changes to tickets/vouch list.

A bot exists under the [gpui-ce org](https://github.com/organizations/gpui-ce/settings/apps/gpuice-vouch-bot) and is installable via https://github.com/apps/gpuice-vouch-bot. The client-id is accessible in the bot settings (first link) and the private key is locked. The former must be configured on this repository as a repository variable under the id `VOUCH_CLIENT_ID`, and the latter as a repository secret under the id `VOUCH_APP_PRIVATE_KEY`. If the bot needs to be installed into additional repositories, it will need to be regenerated and updated in all existing installed repos. Private keys stored in the credential key-pairs, as [documented here](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/managing-private-keys-for-github-apps). The vouch system requires Read access to Discussions (to respond to discussion macros/commands), Read/Write on contents (to write commits with vouch updates), & Read/Write for Issues and Pull Requests (to close/lock them).
