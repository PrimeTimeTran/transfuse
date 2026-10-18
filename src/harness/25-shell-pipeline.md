Requirements:

- Create, update, or edit files only using paths under `00-sanity-tests/`; do not rely on changing directories.

Your first shell command must be `mkdir -p 00-sanity-tests`. Create `00-sanity-tests/25-pipeline-source.txt` with:

```text
apple
banana
apple
cherry
banana
apple
```

Use a CLI pipeline to sort and count identical lines, saving `count fruit` rows to `00-sanity-tests/25-pipeline-result.txt`. Expected:

```text
3 apple
2 banana
1 cherry
```
