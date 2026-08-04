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
                    sh -c "apt-get update -qq && apt-get install -y -qq libudev-dev pkg-config && cargo build --release"
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
