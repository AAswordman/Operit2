# Device-Net Routing and Distribution

## Scope

This document defines the routing and live-data distribution algorithm for a
multi-device Space. It covers the application overlay formed by authenticated
Peer Links. It does not define device roles, execution placement, identity, or
the NAT traversal algorithm used by a carrier such as EasyTier.

EasyTier and a similar carrier select the physical packet path between two
reachable virtual addresses. Operit selects the next authenticated Peer Link
for a Core request and decides where one live source is duplicated. These are
separate decisions and both remain useful when the carrier has established a
direct P2P path.

## Goals

1. Select the least-cost active route rather than merely the route with the
   fewest overlay hops.
2. Send a shared live state source over an edge once, then distribute it at a
   common intermediate node.
3. Preserve the ordering, completion, cancellation, and failure behavior of
   each logical Watch.
4. Make overload visible and isolate a slow subscriber without blocking other
   subscribers.
5. Use only authenticated Space members and active direct Peer Links.

## Link-State Input

Each active directed Peer Link publishes an advertisement:

```text
LinkAdvertisement {
  sourceNodeId
  targetNodeId
  channelEpoch
  sequence
  measuredAt
  expiresAt
  smoothedRttMs
  lossPermille
  congestionPermille
}
```

`channelEpoch` changes whenever a Peer Link is recreated. `sequence` is
monotonic inside that epoch. Receivers retain the newest advertisement for each
directed edge and discard an advertisement at `expiresAt`.

The direct Peer Link measures round-trip time with a sequence-numbered
heartbeat request and echo. It keeps a bounded recent sample window, then
publishes an exponentially weighted moving average. Loss is the fraction of
heartbeat sequence numbers that miss the measurement window. Congestion is the
bounded outbound-frame queue occupancy observed before a frame is accepted by
the carrier.

Advertisements are emitted on a material change and at a fixed refresh
interval. The material-change threshold, integer quantisation, and refresh
period prevent telemetry from turning the persistence synchronisation log into
a heartbeat transport. The Space topology control plane carries the most
recent advertisement to every member.

An edge is eligible only when all of these facts hold:

```text
both endpoints are current Space members
the advertisement has not expired
the local source endpoint is an active direct Peer Link
the edge has a valid channel epoch and sequence
```

The final rule for the local source protects routing from a stale topology
projection. Every remote intermediate node performs the same eligibility check
before forwarding.

## Weighted Route Selection

The router computes a directed shortest-path tree with Dijkstra's algorithm.
The graph contains the eligible advertisements. Edge cost is an integer and is
stable for all nodes that hold the same link-state generation:

```text
cost(u, v) = 1000
           + 10 * smoothedRttMs(u, v)
           + 25 * lossPermille(u, v)
           + 10 * congestionPermille(u, v)
```

The base cost prevents a long chain of individually low-latency links from
winning through accumulated rounding noise. The coefficient values are
configuration constants, not protocol fields. Each route result contains:

```text
RouteDecision {
  destinationNodeId
  nextHopNodeId
  totalCost
  topologyGeneration
}
```

At equal total cost, choose the lexicographically smallest first hop. This
makes independent routers converge on the same path and makes tests
deterministic. A route lookup with no eligible path returns an unreachable
error; it never invents an edge from membership or a stale advertisement.

`RoutedCoreRequest.ttl` remains a hard loop bound. The initial TTL is the
number of eligible vertices, and each forwarding hop reduces it by one.

The router recomputes a route for a new call, Watch open, or Push open. An
opened Watch and Push retain their existing physical path for their lifetime.
This keeps event order intact and avoids silently moving a live stream between
two sources.

## Distribution Classes

A Watch route declares one distribution class in generated Core route metadata:

```text
Unicast
SnapshotShared
```

`Unicast` is used for execution events, tool streams, deltas, append-only
logs, and any sequence where a later subscriber must observe the complete
history. It always opens an independent upstream stream.

`SnapshotShared` is used for a state projection that has a complete current
snapshot and subsequent replacements. A newly attached subscriber receives the
latest cached snapshot before it receives later changes. A route cannot enter
this class merely because two serialized requests look equal.

The generated declaration is the only authority for this classification. The
transport does not infer it from object ids, method names, argument names, or
event payloads.

## Shared Watch Tree

Every router owns a `SharedWatchHub`. A `SnapshotShared` entry is keyed by:

```text
(spaceId, targetNodeId, routeKind, targetObjectId, propertyName,
 canonicalArguments, topologyGeneration)
```

`canonicalArguments` is the structured Link codec representation with the
request id excluded. The Core Value map already has deterministic key order;
the key uses its encoded bytes, not a text search or a substring convention.

The entry owns exactly one upstream Watch and contains:

```text
SharedWatchEntry {
  latestSnapshot
  upstreamSubscription
  downstreamSubscribers
  routeDecision
}
```

The first consumer opens the upstream Watch along the weighted route. A
consumer entering after a snapshot has been stored receives that snapshot on
its own logical subscription id. Changed events are copied to every active
consumer in source order. Completed events close every consumer and remove the
entry. The final consumer cancellation closes the upstream Watch and removes
the entry.

Each downstream subscriber has a bounded event queue. Reaching its bound closes
that logical subscription with `PEER_WATCH_CONSUMER_SLOW`; it does not delay the
source or other consumers. Queue occupancy contributes to the link congestion
measurement used by future route decisions.

The topology formed by these entries is a reverse shortest-path tree rooted at
the source. In this topology:

```text
source A -- B -- C
             \
              D
```

simultaneous equivalent state subscriptions from C and D create one A-to-B
upstream stream and two B-to-consumer streams. A source event crosses A-B once.

## Control Plane and Data Plane

The control plane remains individual and authenticated:

```text
WatchOpen, WatchClose, route errors, and LinkAdvertisement
```

The data plane may be shared only after the `SnapshotShared` declaration has
been checked:

```text
CoreEvent snapshot and changed events
```

Each downstream event is rewritten with that consumer's logical request id.
The original source order is retained within every consumer stream.

## Implementation Boundaries

| Module | Responsibility |
| --- | --- |
| `operit-access-runtime/CoreNodePeerLink` | Heartbeat echo, local measurements, bounded frame queues, per-consumer event delivery. |
| `operit-store/CoreSpaceStore` | One persisted topology record containing direct peers and link metrics, expiry filtering, directed Dijkstra, deterministic route decisions. |
| `operit-node-runtime/CoreNodeRouter` | Select a route for new operations and own the `SharedWatchHub`. |
| `operit-link` and route code generation | Declare `Unicast` or `SnapshotShared` for each true Core Watch route. |

## Verification Topologies

1. A direct high-latency edge and a two-hop low-latency path: Dijkstra chooses
   the lower total cost path.
2. Equal-cost paths: every node chooses the same lexicographically selected
   first hop.
3. An expired advertisement or inactive local Peer Link: the route is absent.
4. C and D subscribe through B to the same `SnapshotShared` source at A: B
   holds one upstream Watch and sends each event once across A-B.
5. A later state subscriber receives the cached snapshot before a subsequent
   changed event.
6. Two `Unicast` subscribers receive independent complete streams.
7. A slow shared subscriber closes without delaying a healthy consumer.
8. A carrier close ends the affected tree branch and leaves unrelated
   subscriptions intact.

## Persistence Synchronization Fan-Out

The persistent synchronization worker first exchanges the authenticated Space
projection through direct pairings. After that exchange, it enumerates every
current Space member that is reachable through the weighted route graph and
performs the normal vector-clock exchange with that member. A multi-hop member
therefore participates in anti-entropy directly through the existing routed
Core call path; it is not required to have an outbound pairing record on the
origin device.

Direct-peer topology and link-quality measurements live in one Space-owned
topology entity. Removing a direct Peer Link removes its topology edge and its
quality measurement. A measurement without a current topology edge cannot
create an eligible route.
