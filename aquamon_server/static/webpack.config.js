const path = require('path');
module.exports = (env, argv) => {
    const mode = argv.mode || 'production';
    console.log("MODE: ", mode);
    console.log(argv.devtool);
    return {
        mode: mode || 'production',
        resolve: {
            extensions: [".ts", ".tsx", ".js", ".jsx"]
        },

        devtool: 'source-map',

        module: {
            rules: [
                {
                    test: /\.ts(x?)$/,
                    exclude: /node_modules/,
                    use: [
                        {
                            loader: "ts-loader"
                        }
                    ]
                },
            ]
        },

        optimization: {
            // split out vendor bundle
            splitChunks: {
                chunks: 'all',
            },
        },

        devServer: {
            contentBase: path.join(__dirname, ''),
            publicPath: '/',
            port: 9002,
            proxy: [{
                context: ['/api'],
                target: "http://192.168.1.243",
                secure: false,
            }]
        },

    }
}