# Contributing to Kentik Labs Projects

Want to contribute?  Awesome!

Please follow the guidelines below to ensure healthy and productive participation.

# Reporting Issues

Reporting issues and errors is a great way to contribute.  We appreciate well-written thorough issue reports as detailed as possible.

Please check existing [Issues](/issues) before submitting.  If an existing issue is there, you can use the "subscribe" button to get updates.  If you have something to add please do so but "+1" or "same" comments typically clutter the discussion and do not really help.

When opening a new issue, please include the version you are running and ways to reproduce the problem if possible.  If there is a long log file or attachment, please use a gist (https://gist.github.com).  Please check and remove any sensitive information from the log before posting.

# Contributions

This section will help contributors to the project.

## Pull Requests

Every pull request is appreciated!  No matter if it is a typo, documentation, instructions, or code we welcome every pull request.  If it is a significant feature ore refactor, please open an issue to discuss before spending time on the change to ensure that the maintainers are in agreement on the direction.


## Connect

To connect with other contributors we have a [Discord](https://discord.gg/kentik) server setup
for more realtime discussion.

## Conventions

Fork the repository and make changes on your fork in a feature branch:

- If it's a bug fix branch, name it XXXX-something where XXXX is the number of
	the issue.
- If it's a feature branch, create an enhancement issue to announce
	your intentions, and name it XXXX-something where XXXX is the number of the
	issue.

### Documentation

Please watch the pull request for test (CI) results and address any failures.  Also please
ensure to update the documentation when creating or modifying features.

### Commit Messages

Please include a short summary (max 50 chars) followed by an optional more detailed explanation
separated from the summary by an empty line.  This helps keep the version history clean.

Commit messages should follow best practices, including explaining the context
of the problem and how it was solved, including in caveats or follow up changes
required. They should tell the story of the change and provide readers understanding
of what led to it.  If you are completely new, please see
[How to Write a Git Commit Message](http://chris.beams.io/posts/git-commit/) for a start.

### Review

Code review comments may be added to your pull request. Discuss, then make the
suggested modifications and push additional commits to your feature branch. Post
a comment after pushing. New commits show up in the pull request automatically,
but the reviewers are notified only when you comment.

Pull requests must be cleanly rebased on top of master without multiple branches
mixed into the PR.

Before you make a pull request, squash your commits into logical units of work
using `git rebase -i` and `git push -f`. A logical unit of work is a consistent
set of patches that should be reviewed together: for example, upgrading the
version of a vendored dependency and taking advantage of its now available new
feature constitute two separate units of work. Implementing a new function and
calling it in another file constitute a single logical unit of work. The very
high majority of submissions should have a single commit, so if in doubt: squash
down to one.

# Community Guidelines

We want to keep the community awesome.  Please follow these guidelines:

* Be courteous and respectful to fellow community members.  No racial, gender, or other
abuse will be tolerated.

* Encourage participation from all.  Please make everyone feel welcome in the community
regardless of background and do everything possible to encourage participation.

