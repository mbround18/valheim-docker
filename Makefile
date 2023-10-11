# Following this tutorial: https://markentier.tech/posts/2022/01/speedy-rust-builds-under-wsl2/
# This makes developing on windows significantly easier for rust projects!!

SOURCE_DIR = $(PWD)
# `notdir` returns the part after the last `/`
# so if the source was "/some/nested/project", only "project" remains
BUILD_DIR  = ~/tmp/$(notdir $(SOURCE_DIR))

wsl.build: wsl.sync
	cd $(BUILD_DIR) && cargo build
	rsync -av $(BUILD_DIR)/target/debug/ $(SOURCE_DIR)/target/debug/ \
		--exclude .git \
		--exclude target \
		--exclude .fingerprint \
		--exclude build \
		--exclude incremental \
		--exclude deps

wsl.run: wsl.sync
	cd $(BUILD_DIR) && cargo run

wsl.test: wsl.sync
	cd $(BUILD_DIR) && cargo test

wsl.sync:
	mkdir -p $(BUILD_DIR)
	rsync -av $(SOURCE_DIR)/ $(BUILD_DIR)/ --exclude .git --exclude target --exclude tmp

wsl.clean:
	rm -rf $(BUILD_DIR)/target

wsl.clean-all:
	rm -rf $(BUILD_DIR)

wsl.clippy: wsl.sync
	cd $(BUILD_DIR) && cargo clippy
