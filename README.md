

## Docker
### Listener mode
```
docker run --rm -v $PWD:/out -w /out --net host --cap-add=NET_ADMIN --cap-add NET_RAW seg scan --targets /out/target.txt --protocol both
```

### Scanner mode
```
docker run -it --rm -v $PWD:/out -w /out --net host --cap-add=NET_ADMIN --cap-add NET_RAW seg listen --protocol both
```
