# Replicante Agent for MongoDB

The Replicante project is a control plane for stateful systems like databases.
Agents are the interfaces that enable and standardise centralised and automated management.

This repository hosts the Replicante Agent for [MongoDB] server.

## Usage

The agent process should be run on the same host as the [MongoDB] server process.
It access the server using a [MongoDB] client connection, by default on localhost.

### Supported Modes

Since [MongoDB] can be run in many different configurations some
agent's operations will vary based on how [MongoDB] is running.
For example adding a node to a replica set is a very different operation to adding a
node to a sharded cluster.

To handle these difference the [MongoDB] agent can run in different modes:

- `repliagent-mongodb replicaset`: run the agent to manage a Replica Set member node
  (arbiter nodes are NOT supported).

### Configuration

Once you know the mode to run the agent with, the agent may need some required configuration:

<!-- markdownlint-disable MD013 --->
| Description | Config File Option | Environment Variable | Mode(s) |
| - | - | - | - |
| Address to connect to the agent-local [MongoDB] process | `addresses.cluster` | `RA_ADDRESS_CLUSTER` | Replica Set |
| Platform ID of the node running the agent and [MongoDB] process | `node_id` | `RA_NODE_ID` | Replica Set |
<!-- markdownlint-enable MD013 --->

Aside from the required options mentioned above there are more options available.
These can be set in the agent configuration file, an example
of which is in the `mongoagent.example.yaml` file.

## Supported MongoDB Versions

This agent is compatible with MongoDB version 3.6 and grater.

## Available Agent Actions

- Standard `agent.replicante.io/test.*` actions.
- Cluster actions:
  - `agent.replicante.io/cluster.add` to add nodes to RS.
    - `id: Option<u32>`: Replica Set member `_id` for the new node.
    - `host: String`: The `host` of the new Replica Set member to add.
  - `agent.replicante.io/cluster.init` to initialise a single-node Replica Set.
    - `settings: Option<bson::Document>`: settings passed to the `replSetInitiate` command.

[MongoDB]: https://www.mongodb.com/
