pipeline {
    agent any

    environment {
        GITHUB_TOKEN = credentials('GITHUB_TOKEN')
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

        stage('Install Frontend Deps') {
            steps {
                sh '''
                cd frontend
                npm ci
                '''
            }
        }

        stage('Lint') {
            steps {
                catchError(buildResult: 'FAILURE', stageResult: 'FAILURE') {
                    sh '''
                    cd frontend
                    npm run lint
                    '''
                }
                catchError(buildResult: 'FAILURE', stageResult: 'FAILURE') {
                    sh '''
                    cargo clippy -- -D clippy::pedantic
                    '''
                }
            }
        }

        stage('Format Check') {
            steps {
                catchError(buildResult: 'FAILURE', stageResult: 'FAILURE') {
                    sh '''
                    cd frontend
                    npm run format:check
                    '''
                }
                catchError(buildResult: 'FAILURE', stageResult: 'FAILURE') {
                    sh '''
                    cargo fmt -- --check
                    '''
                }
            }
        }

        stage('Build') {
            steps {
                sh '''
                cd frontend
                npm run build
                '''
                sh '''
                cargo build --release
                '''
            }
        }

        stage('Test') {
            steps {
                sh '''
                cargo test
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
