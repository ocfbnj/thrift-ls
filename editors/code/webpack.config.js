const path = require("path");
const CopyWebpackPlugin = require('copy-webpack-plugin');

module.exports = {
    target: "node",
    entry: {
        main: "./src/main.ts",
        server: "./src/server.ts"
    },
    output: {
        path: path.resolve(__dirname, "out"),
        filename: "[name].js",
        libraryTarget: "commonjs",
    },
    mode: "production",
    externals: {
        vscode: "commonjs vscode",
    },
    experiments: {
        asyncWebAssembly: true,
    },
    resolve: {
        extensions: ['.ts', '.js'],
    },
    module: {
        rules: [
            {
                test: /\.ts$/,
                exclude: /node_modules/,
                use: [
                    {
                        loader: 'ts-loader'
                    }
                ]
            }
        ]
    },
    plugins: [
        new CopyWebpackPlugin({
            patterns: [
                {
                    from: path.resolve(__dirname, '../../pkg/thrift_analyzer_bg.wasm'),
                    to: path.resolve(__dirname, 'out')
                }
            ]
        })
    ]
};
