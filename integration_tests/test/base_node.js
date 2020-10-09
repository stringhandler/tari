var assert = require('assert');




var grpc = require('grpc');
var protoLoader = require('@grpc/proto-loader');

var PROTO_PATH = __dirname + '/../../applications/tari_app_grpc/proto/base_node.proto';
// Suggested options for similarity to existing grpc.load behavior
var packageDefinition = protoLoader.loadSync(
    PROTO_PATH,
    {keepCase: true,
        longs: String,
        enums: String,
        defaults: true,
        oneofs: true
    });
var protoDescriptor = grpc.loadPackageDefinition(packageDefinition);
console.log(protoDescriptor);
// The protoDescriptor object has the full package hierarchy
var tari = protoDescriptor.tari.rpc;
var client = new tari.BaseNode('127.0.0.1:50051', grpc.credentials.createInsecure());


var WALLET_PROTO_PATH = __dirname + '/../../applications/tari_app_grpc/proto/wallet.proto';
var packageDefinition2 = protoLoader.loadSync(
    WALLET_PROTO_PATH,
    {keepCase: true,
        longs: String,
        enums: String,
        defaults: true,
        oneofs: true
    });
var protoDescriptor2 = grpc.loadPackageDefinition(packageDefinition2);
console.log(protoDescriptor2);
// The protoDescriptor object has the full package hierarchy
var tariWallet = protoDescriptor2.tari.rpc;
var walletClient = new tariWallet.Wallet('127.0.0.1:50061', grpc.credentials.createInsecure());

console.log(client);

describe('Base Node', function () {
    this.timeout(10000); // five minutes
    describe('GetVersion', function () {
        it('should return', function (done) {
            var listHeaders = {
                "from_height": 100,
                    "num_headers": 100,
                    "sorting": 0
            };
            client.getVersion({}, function(err, constants) {
                console.log("returned");
                console.log(constants);
                if (err) {
                    done(err);
                }
                else {
                    assert.equal([1, 2, 3].indexOf(4), -1);
                    done();
                }

            })
        });
    });

    describe('GetBlockTemplate', function() {
        it('Should return', function(done) {
            client.getNewBlockTemplate({}, function(err, result) {
                console.log(err);
                if (err) {
                    return done(err);
                }

                console.log(result);
                done();
            })
        })
    });

    describe('Miner', function(){
        it('As a miner I want to mine a block', function(done) {
         client.getNewBlockTemplate({}, function(err, template){
             if (err) {
                 return done(err);
             }
             console.log(template);
             var block = template.new_block_template;
             walletClient.getCoinbase({
                 "reward": template.block_reward,
                 "fee": 0,
                 "height": block.header.height
             }, function(err, coinbase) {
                 if (err) {
                     return done(err);
                 }
                 console.log(coinbase);
                 var cb= coinbase.transaction;
                 block.body.outputs = block.body.outputs.concat(cb.body.outputs);
                 block.body.kernels = block.body.kernels.concat(cb.body.kernels);
                 client.getNewBlock(block, function(err, b){
                    if (err) {
                        return done(err);
                    }
                    console.log(b);
                    client.submitBlock(b.block, function(err, empty){
                        if (err) {
                            return done(err);
                        }
                        console.log(empty);
                        done();
                    })
                 })
             });
         })
        })
    } )
});
