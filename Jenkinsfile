pipeline {
    agent any

    environment {
        GITHUB_TOKEN = credentials('GITHUB_TOKEN')
        DOCKER_VOLS = '-v jenkins_jenkins_home:/var/jenkins_home -v cargo-registry-cache:/usr/local/cargo/registry'
        NODE_IMAGE = 'node:22'
        RUST_IMAGE = 'rust:latest'
    }

    stages {
        stage('Checkout') {
            steps {
                checkout scm
                sh '''
                curl -L \
                -X POST \
                -H "Accept: application/vnd.github+json" \
                -H "Authorization: Bearer $GITHUB_TOKEN" \
                -H "X-GitHub-Api-Version: 2022-11-28" \
                https://api.github.com/repos/tanndlin/open-deck/statuses/$GIT_COMMIT \
                -d '{"state":"pending","description":"Build in progress","context":"Jenkins"}'
                '''
            }
        }

        // Builds frontend/dist first: the Rust backend embeds it at compile
        // time via rust-embed, so every later cargo command needs it present.
        stage('Install & Build Frontend') {
            steps {
                sh '''
                docker run --rm $DOCKER_VOLS -w $WORKSPACE/frontend $NODE_IMAGE \
                    sh -c "npm ci && npm run build"
                '''
            }
        }

        stage('Lint') {
            steps {
                catchError(buildResult: 'FAILURE', stageResult: 'FAILURE') {
                    sh '''
                    docker run --rm $DOCKER_VOLS -w $WORKSPACE/frontend $NODE_IMAGE \
                        sh -c "npm run lint"
                    '''
                }
                catchError(buildResult: 'FAILURE', stageResult: 'FAILURE') {
                    sh '''
                    docker run --rm $DOCKER_VOLS -w $WORKSPACE -e DEBIAN_FRONTEND=noninteractive $RUST_IMAGE \
                        sh -c "apt-get update -qq && apt-get install -y -qq libudev-dev pkg-config && rustup component add clippy && cargo clippy --all-targets -- -D clippy::pedantic"
                    '''
                }
            }
        }

        stage('Format Check') {
            steps {
                catchError(buildResult: 'FAILURE', stageResult: 'FAILURE') {
                    sh '''
                    docker run --rm $DOCKER_VOLS -w $WORKSPACE/frontend $NODE_IMAGE \
                        sh -c "npm run format:check"
                    '''
                }
                catchError(buildResult: 'FAILURE', stageResult: 'FAILURE') {
                    sh '''
                    docker run --rm $DOCKER_VOLS -w $WORKSPACE $RUST_IMAGE \
                        sh -c "rustup component add rustfmt && cargo fmt -- --check"
                    '''
                }
            }
        }

        stage('Build') {
            steps {
                sh '''
                docker run --rm $DOCKER_VOLS -w $WORKSPACE -e DEBIAN_FRONTEND=noninteractive $RUST_IMAGE \
                    sh -c "apt-get update -qq && apt-get install -y -qq libudev-dev pkg-config gcc-mingw-w64-x86-64 && \
                        rustup target add x86_64-pc-windows-gnu && \
                        cargo build --release && \
                        cargo build --release --target x86_64-pc-windows-gnu"
                '''
            }
        }

        stage('Test') {
            steps {
                sh '''
                docker run --rm $DOCKER_VOLS -w $WORKSPACE -e DEBIAN_FRONTEND=noninteractive $RUST_IMAGE \
                    sh -c "apt-get update -qq && apt-get install -y -qq libudev-dev pkg-config && cargo test"
                '''
            }
        }

        // Rolls the "latest" GitHub release forward to this commit's binary,
        // giving a stable download URL that always points at the newest build.
        stage('Publish Latest Release') {
            when {
                expression { env.GIT_BRANCH == 'master' || env.GIT_BRANCH == 'origin/master' }
            }
            steps {
                sh '''
                docker run --rm $DOCKER_VOLS -w $WORKSPACE \
                    -e GITHUB_TOKEN=$GITHUB_TOKEN -e GIT_COMMIT=$GIT_COMMIT \
                    alpine:3.20 \
                    sh -c "apk add --no-cache curl jq >/dev/null && sh scripts/publish-latest-release.sh"
                '''
            }
        }
    }

    post {
        success {
            sh '''
            curl -L \
            -X POST \
            -H "Accept: application/vnd.github+json" \
            -H "Authorization: Bearer $GITHUB_TOKEN" \
            -H "X-GitHub-Api-Version: 2022-11-28" \
            https://api.github.com/repos/tanndlin/open-deck/statuses/$GIT_COMMIT \
            -d '{"state":"success","description":"Build succeeded","context":"Jenkins"}'
            '''
        }
        failure {
            sh '''
            curl -L \
            -X POST \
            -H "Accept: application/vnd.github+json" \
            -H "Authorization: Bearer $GITHUB_TOKEN" \
            -H "X-GitHub-Api-Version: 2022-11-28" \
            https://api.github.com/repos/tanndlin/open-deck/statuses/$GIT_COMMIT \
            -d '{"state":"failure","description":"Build failed","context":"Jenkins"}'
            '''
        }
    }
}
