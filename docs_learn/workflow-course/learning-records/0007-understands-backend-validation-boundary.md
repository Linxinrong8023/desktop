# Understands why backend graph validation is mandatory

The learner correctly explained that imported workflow data can bypass React Flow interaction guards, so backend graph parsing remains mandatory even when the editor prevents invalid gestures. This establishes the frontend-as-UX versus backend-as-trust-boundary distinction.

## Evidence

When asked why backend validation duplicates frontend checks, the learner immediately identified workflow import as a path that does not pass through the normal editor controls and could otherwise admit an invalid graph.

## Implications

Future lessons can extend this boundary to direct API calls, older clients, malformed persisted data, and validation timing without reteaching why client-side validation is insufficient.
