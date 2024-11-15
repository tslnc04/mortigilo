# mortigilo

> Sidecar container to restart qbittorrent automatically

## Motivation

For whatever reason, qbittorrent seems to have seeding slow down and eventually stop working after a few days. Restarting the container always fixes the issue, so this is a small sidecar container that uses the qbittorrent API to restart the container based on a provided condition.

[qbittorrent/qBittorrent#20722] is one such issue, although in this case the reporter says it was resolved in 5.0.0. Unfortunately, it still is an issue for me.

[qbittorrent/qBittorrent#20722]: https://github.com/qbittorrent/qBittorrent/issues/20722

## Usage

The container exposes an endpoint at `/healthz` that returns `204 No Content` if the condition is considered healthy, or `503 Service Unavailable` if the condition is not met. This is intended to be used with a Kubernetes liveness probe. Since pods share networking, as long as the mortigilo container is running in the same pod, its endpoint can be used for the qbittorrent container's liveness probe.

As a bonus, there is a readiness probe at `/readyz` that returns `204 No Content` if the qbittorrent instance is reachable and is either connected or firewalled. If the instance is not reachable or disconnected, the readiness probe will return `503 Service Unavailable`.

### Kubernetes

For running as a sidecar container (beta feature behind flag in Kubernetes 1.29+):

```yaml
initContainers:
  - name: mortigilo
    image: ghcr.io/tslnc04/mortigilo:latest
    restartPolicy: Always
    env:
      - name: QBITTORRENT_HOST
        value: http://localhost:8080
      - name: QBITTORRENT_USERNAME
        value: admin
      - name: QBITTORRENT_PASSWORD
        valueFrom:
          secretKeyRef:
            name: qbittorrent-login
            key: password
      - name: PORT
        value: "9000"
      - name: ADDRESS
        value: "0.0.0.0"
    ports:
      - containerPort: 9000
containers:
  - name: qbittorrent
    # ... other container configuration
    livenessProbe:
      httpGet:
        path: /healthz
        port: 9000
      initialDelaySeconds: 60
    readinessProbe:
      httpGet:
        path: /readyz
        port: 9000
```

If not running as a sidecar container, the mortigilo container can be used under `containers` and `restartPolicy` does not need to be set.

## Config

| Environment variable | Default                 | Description                             |
| -------------------- | ----------------------- | --------------------------------------- |
| QBITTORRENT_HOST     | `http://localhost:8080` | Host of the qbittorrent instance        |
| QBITTORRENT_USERNAME | `admin`                 | Username of the qbittorrent user        |
| QBITTORRENT_PASSWORD |                         | Password of the qbittorrent user        |
| PORT                 | `9000`                  | Port to serve the probe endpoints on    |
| ADDRESS              | `0.0.0.0`               | Address to serve the probe endpoints on |

Note that the transport of the host is important. If `http` is specified but it redirects to `https`, the container will not be able to log into the qbittorrent API and all requests will fail.

## Determining health

Currently, there is no way to configure what qualifies as healthy. Instead, it is hard coded to consider the qbittorrent instance unhealthy if all active torrents are stalled and both the download and upload speeds are zero.

Originally this was intended to be configurable but it defeats the dead simple purpose of the container, so the recommended approach to change the behavior is to modify the source code.

## Name

Mortigilo is an Esperanto word meaning "killer" or more literally "tool that causes death". The less morbid reason for this name is that the container is intended to determine whether a qbittorrent instance should be killed.

Mi decidis uzi la nomon antaŭ ol la projekto estis farita tiel ĝi iomete malpravas ĉar la programo ne mortigas nenio rekte.

## Copyright

Copyright 2024 Kirsten Laskoski

Licensed under the MIT License. See [LICENSE](./LICENSE) for details.
