# goose Release Manual Testing Checklist

## Use the following script to create a risk assessment and testing plan:
```
./workflow_recipes/release_risk_check/run.sh {{VERSION}}
```

It will generate an analysis report in `/tmp/release_report_final.md` and perform testing is necessary for high risk pr changes.

## Run the goose self-test recipe

goose run --recipe goose-self-test.yaml
