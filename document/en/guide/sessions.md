# Sessions

How to save, load, and manage captured traffic data in Cheolsu Proxy.

---

## What is a Session?

A session is a collection of captured network traffic. Every time you use Cheolsu Proxy, you are working within a session — it holds all the HTTP transactions, WebSocket frames, and metadata that the proxy has recorded.

Sessions are useful when you need to:

- **Save a debugging session** to revisit later without re-capturing the traffic
- **Share captured traffic** with a teammate for collaborative debugging
- **Compare traffic** from different test runs or environments
- **Archive evidence** of a specific network behavior before clearing the view

## Saving a Session

When you have captured traffic that you want to preserve:

1. Go to the session management area in the application
2. Save the current session — this writes all captured traffic to disk
3. Give the session a meaningful name so you can identify it later

Saved sessions include the full request and response data (headers, bodies, timing) for every captured transaction.

## Loading a Session

To revisit previously captured traffic:

1. Open the session management area
2. Browse your saved sessions
3. Select and load the session you want to review

The loaded session's traffic appears in the traffic list, just as if you had captured it live. You can inspect individual requests, apply filters with [Cheolsu-Query](../features/cheolsu-query.md), and export to HAR — all the same tools work on loaded sessions.

## Managing Sessions

### Organizing Sessions

Over time you may accumulate many saved sessions. Keep things manageable by:

- Using descriptive names that include the date, feature, or bug number (e.g., "payment-api-debug-2026-03-10")
- Deleting sessions you no longer need to free up disk space

### Clearing the Current Session

To clear all traffic from the current active session without saving:

1. Use the clear/reset action in the application
2. All captured traffic is removed from memory
3. The proxy continues running and will capture new traffic into a fresh session

This is useful when your traffic list has grown large and you want a clean slate without restarting the proxy.

## Tips

### Memory Management During Long Captures

Each captured request consumes memory. During long-running capture sessions — especially when monitoring high-traffic applications — memory usage can grow significantly. To keep things under control:

- **Clear periodically** — If you do not need to retain all historical traffic, clear the session at regular intervals
- **Use filters wisely** — Filtering does not reduce memory usage (all traffic is still stored), but it helps you find what you need faster so you can clear sooner
- **Save and clear** — If you want to preserve the data but free up memory, save the session to disk and then clear the active session
- **Export to HAR** — For archival purposes, exporting to HAR and clearing is often more practical than keeping sessions loaded

### Session Data Location

Session files are stored locally on your machine. The exact location depends on your operating system:

- **macOS**: `~/Library/Application Support/CheolsuProxy/`

## Related Documentation

- [Recording](./recording.md) — How to capture and inspect traffic
- [Basic Usage](./basic-usage.md) — Quick-start overview
- [Troubleshooting](./troubleshooting.md) — Common issues and solutions

---

**Next step**: Understand how your traffic reaches the proxy in the [Proxying](./proxying.md) guide.
