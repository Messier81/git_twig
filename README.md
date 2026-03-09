# git-twig (`gt`)

Stacked branches + parallel worktrees for the AI agent age.

`gt` combines the stacked branch workflow (like [git-spice](https://github.com/abhinav/git-spice)) with Git worktrees, giving every branch its own directory. Multiple AI agents — or developers — can work on dependent features in parallel without stepping on each other's files.

## Why

In a normal Git workflow, you have one directory and switch between branches. If feature B depends on feature A, you either wait for A to merge or juggle checkouts and stashes.

With `gt`:
- Every branch gets its own worktree (directory), so you can work on multiple branches simultaneously
- Branches can be stacked — feature B builds on feature A's code right away, no waiting for merge
- One command restacks everything when a parent branch changes
- Each branch becomes a small, reviewable PR

```
● main (trunk)
├── ○  feat-api          ~/project.feat-api
│   └── ○  feat-api-routes   ~/project.feat-api-routes
├── ○  feat-auth         ~/project.feat-auth
│   └── ○  feat-auth-ui      ~/project.feat-auth-ui
└── ○  feat-db           ~/project.feat-db
```

## Install

Requires [Rust](https://rustup.rs/).

```bash
cargo install --git https://github.com/Messier81/git_twig
```

Then install shell integration (required for navigation commands to `cd` between worktrees):

```bash
gt shell install
source ~/.zshrc
```

## Quick start

```bash
cd your-repo
gt init                     # initialize gt (detects trunk branch)

gt b c feat-auth            # create branch + worktree, stacked on current branch
gt b c feat-auth-ui         # run from feat-auth → stacks on top of it

gt s                        # show the tree
gt d                        # move to next worktree
gt u                        # move to previous worktree
gt sw feat-auth             # jump directly to a branch (tab completion works)

gt lg                       # log: show commits unique to this branch
gt rs                       # restack: rebase all branches onto their parents
gt sy                       # sync: pull latest trunk + restack
gt su                       # submit: push all branches + create/update stacked PRs

gt b d feat-auth-ui         # delete branch + worktree (with confirmation)
gt b d feat-auth-ui -f      # skip confirmation
```

## Commands

| Command | Alias | Description |
|---------|-------|-------------|
| `gt init` | `gt i` | Initialize gt in the current repo |
| `gt branch create <name>` | `gt b c` | Create a branch + worktree stacked on the current branch |
| `gt branch delete <name>` | `gt b d` | Delete a branch + worktree, re-parent children |
| `gt status` | `gt s` | Show branch tree with worktree paths |
| `gt up` | `gt u` | Move to the previous worktree (depth-first order) |
| `gt down` | `gt d` | Move to the next worktree (depth-first order) |
| `gt switch <name>` | `gt sw` | Jump to a specific branch's worktree |
| `gt log` | `gt lg` | Show commits unique to the current branch |
| `gt restack` | `gt rs` | Rebase all branches onto their parents |
| `gt sync` | `gt sy` | Pull latest trunk from remote and restack |
| `gt submit` | `gt su` | Push all branches and create/update stacked PRs |
| `gt shell install` | | Install shell integration (zsh/bash) |
| `gt shell uninstall` | | Remove shell integration |

## How it works

`gt` manages three things:

1. **Git branches** — your actual code (commits, PRs)
2. **Git worktrees** — separate directories so each branch can be checked out simultaneously
3. **Stack relationships** — which branch depends on which, stored in `.git/gt/state.json`

When you run `gt b c feat-auth-ui` from inside `feat-auth`, gt creates a new branch, gives it its own worktree directory, and records that `feat-auth-ui` is stacked on `feat-auth`. Later, when `feat-auth` gets new commits, `gt rs` rebases `feat-auth-ui` on top automatically.

All worktrees are flat sibling directories on disk:

```
~/project/                   ← main
~/project.feat-auth/         ← feat-auth
~/project.feat-auth-ui/      ← feat-auth-ui
~/project.feat-db/           ← feat-db
```

## Stacked PRs

`gt su` pushes every tracked branch and creates a PR for each one, with the base set to its parent branch. This gives you a chain of small, incremental PRs instead of one massive diff:

```
PR #1: feat-auth → main           (just the auth service)
PR #2: feat-auth-ui → feat-auth   (just the login page, diff only shows UI code)
PR #3: feat-db → main             (just the database layer)
```

If PRs already exist, `gt su` updates their base branch and pushes new commits. Requires [GitHub CLI](https://cli.github.com) (`gh`).

## Shell integration

Navigation commands (`gt u`, `gt d`, `gt sw`) need to `cd` your shell into a different directory. Since a subprocess can't change the parent shell's directory, `gt shell install` adds a thin shell function to your `~/.zshrc` (or `~/.bashrc`) that intercepts these commands and performs the `cd`. It also sets up tab completion.

## License

MIT
