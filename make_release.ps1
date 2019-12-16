Get-ChildItem "Cargo.toml" | ForEach-Object {
	$conf = $_ | Get-Content -raw
	$conf -match 'version\s+=\s+"(.*)"' | out-null
	$script:TIGER_VERSION = $matches[1]
}

git tag -a $TIGER_VERSION -m "Tiger $TIGER_VERSION"
git push origin $TIGER_VERSION
