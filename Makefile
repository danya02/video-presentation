publish:
	trunk build --release
	ssh rudnlab "rm -r ~/presentation/dist/*"
	scp -r ./dist rudnlab:~/presentation