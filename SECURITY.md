# Security

## Scope and trust model

`dkr` is a local developer tool: it reads YAML profile files you author and
turns them into a `docker run` invocation with the same privileges you
already have on your machine. A profile is as trusted as a shell script —
don't run a profile you didn't write or review, the same way you wouldn't
run an unreviewed shell script.

Profile YAML commonly contains environment variables. Treat any profile file
containing secrets (API keys, tokens) the same way you'd treat a `.env` file:
don't commit it to a public repository, and prefer `$VAR`/`${VAR}` expansion
(pulling the actual secret from your shell environment or a secret manager)
over hardcoding it in the profile.

## Reporting a vulnerability

Open a GitHub issue, or email jukka.nousiainen@gmail.com for anything you'd
rather not post publicly. There's no fixed response-time SLA — this is a
personal project — but reports are welcome and will be looked at.
