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
                {
                    test: /\.less$/,
                    use: [
                      'style-loader',
                      {
                        loader: "css-loader",
                        options: {
                            sourceMap: mode !== 'production',
                            modules: {
                                localIdentName: mode === 'production' ? '[hash:base64]' : "[local]___[hash:base64:5]",
                            },
                            url: false,     // TODO: this disables inline/load images, for now. Is it worth it to do this?
                            importLoaders: 1,
                            localsConvention: 'camelCase',
                        }
                      },
                      {
                          loader: 'less-loader',
                          options: { sourceMap: mode != 'production' }
                      }
                    ]
                  }
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