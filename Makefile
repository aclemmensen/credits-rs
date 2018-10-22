codegen:
	protoc --rust_out=src --grpc_out=src --plugin=protoc-gen-grpc=$(shell which grpc_rust_plugin) credits.proto

grpcc:
	docker run --rm -it -v ${CURDIR}:/proto --net=host therealplato/grpcc-container grpcc -p /proto/credits.proto -i --address localhost:5951