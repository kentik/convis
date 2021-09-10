# convis - Container visibility

convis demonstrates use of the Linux extended BPF facility to
attribute process and container information to network traffic.

Usage:

```
cargo build --release
sudo target/release/convis -v
```

## Sinks

Convis can output metrics to New Relic and Prometheus in addition to stdout. 

* Target New Relic: `./convis --sink newrelic,account=$NR_ACCOUNT_ID,key=$NR_INSIGHTS_INSERT_KEY`
* Target Grafana Cloud: `./convis --sink 'prometheus,endpoint=https://$PROMETHEUS_HOST.grafana.net/api/prom/push,username=$PROMETHEUS_ID,password=$GRAFANA_API_KEY'`
  

## License

Copyright 2021 Kentik, Inc.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
