pipeline {
    agent any

    environment {
        REGISTRY        = 'ghcr.io'
        IMAGE_PREFIX    = "${REGISTRY}/rust-conductor"
        BACKEND_IMAGE   = "${IMAGE_PREFIX}/backend"
        FRONTEND_IMAGE  = "${IMAGE_PREFIX}/frontend"
        HELM_RELEASE    = 'rust-conductor'
        CARGO_TERM_COLOR = 'always'
    }

    options {
        buildDiscarder(logRotator(numToKeepStr: '20'))
        timestamps()
        timeout(time: 30, unit: 'MINUTES')
        disableConcurrentBuilds()
    }

    parameters {
        choice(name: 'DEPLOY_ENV', choices: ['none', 'staging', 'production'], description: 'Target environment')
        booleanParam(name: 'SKIP_TESTS', defaultValue: false, description: 'Skip test stage')
        booleanParam(name: 'FORCE_DEPLOY', defaultValue: false, description: 'Force deploy even on non-main branch')
    }

    stages {
        // ── Lint & Format ───────────────────────────────────────────
        stage('Lint') {
            parallel {
                stage('Rust Format & Clippy') {
                    agent {
                        docker {
                            image 'rust:1.94-slim'
                            args '--user root'
                        }
                    }
                    steps {
                        sh 'apt-get update && apt-get install -y protobuf-compiler pkg-config libssl-dev'
                        dir('backend') {
                            sh 'rustup component add rustfmt clippy'
                            sh 'cargo fmt --all -- --check'
                            sh 'cargo clippy --all-targets --all-features -- -D warnings'
                        }
                    }
                }
                stage('Frontend Lint') {
                    agent {
                        docker {
                            image 'node:24-alpine'
                        }
                    }
                    steps {
                        dir('frontend') {
                            sh 'npm ci'
                            sh 'npm run lint'
                        }
                    }
                }
            }
        }

        // ── Test ────────────────────────────────────────────────────
        stage('Test') {
            when {
                expression { return !params.SKIP_TESTS }
            }
            agent {
                docker {
                    image 'rust:1.94-slim'
                    args '--user root'
                }
            }
            steps {
                sh 'apt-get update && apt-get install -y protobuf-compiler pkg-config libssl-dev'
                dir('backend') {
                    sh 'cargo test --bins 2>&1 | tee test-results.txt'
                }
            }
            post {
                always {
                    archiveArtifacts artifacts: 'backend/test-results.txt', allowEmptyArchive: true
                }
            }
        }

        // ── Build Docker Images ─────────────────────────────────────
        stage('Build Images') {
            parallel {
                stage('Build Backend') {
                    steps {
                        script {
                            def tag = env.GIT_COMMIT?.take(7) ?: 'latest'
                            docker.build("${BACKEND_IMAGE}:${tag}", '--build-arg CARGO_FEATURES=default ./backend')
                            // Also tag as branch name
                            sh "docker tag ${BACKEND_IMAGE}:${tag} ${BACKEND_IMAGE}:${env.BRANCH_NAME}"
                        }
                    }
                }
                stage('Build Frontend') {
                    steps {
                        script {
                            def tag = env.GIT_COMMIT?.take(7) ?: 'latest'
                            docker.build("${FRONTEND_IMAGE}:${tag}", './frontend')
                            sh "docker tag ${FRONTEND_IMAGE}:${tag} ${FRONTEND_IMAGE}:${env.BRANCH_NAME}"
                        }
                    }
                }
            }
        }

        // ── Security Scan ───────────────────────────────────────────
        stage('Security Scan') {
            steps {
                script {
                    def tag = env.GIT_COMMIT?.take(7) ?: 'latest'
                    // Trivy scan for both images
                    sh """
                        docker run --rm \
                          -v /var/run/docker.sock:/var/run/docker.sock \
                          aquasec/trivy:latest image \
                          --severity CRITICAL,HIGH \
                          --exit-code 0 \
                          --format table \
                          ${BACKEND_IMAGE}:${tag}
                    """
                    sh """
                        docker run --rm \
                          -v /var/run/docker.sock:/var/run/docker.sock \
                          aquasec/trivy:latest image \
                          --severity CRITICAL,HIGH \
                          --exit-code 0 \
                          --format table \
                          ${FRONTEND_IMAGE}:${tag}
                    """
                }
            }
        }

        // ── Push Images ─────────────────────────────────────────────
        stage('Push Images') {
            when {
                anyOf {
                    branch 'main'
                    branch 'develop'
                    expression { return params.FORCE_DEPLOY }
                }
            }
            steps {
                script {
                    def tag = env.GIT_COMMIT?.take(7) ?: 'latest'
                    docker.withRegistry("https://${REGISTRY}", 'ghcr-credentials') {
                        docker.image("${BACKEND_IMAGE}:${tag}").push()
                        docker.image("${BACKEND_IMAGE}:${tag}").push(env.BRANCH_NAME)
                        docker.image("${FRONTEND_IMAGE}:${tag}").push()
                        docker.image("${FRONTEND_IMAGE}:${tag}").push(env.BRANCH_NAME)
                        if (env.BRANCH_NAME == 'main') {
                            docker.image("${BACKEND_IMAGE}:${tag}").push('latest')
                            docker.image("${FRONTEND_IMAGE}:${tag}").push('latest')
                        }
                    }
                }
            }
        }

        // ── Deploy Staging ──────────────────────────────────────────
        stage('Deploy Staging') {
            when {
                anyOf {
                    allOf {
                        branch 'main'
                        expression { return params.DEPLOY_ENV != 'none' }
                    }
                    expression { return params.DEPLOY_ENV == 'staging' && params.FORCE_DEPLOY }
                }
            }
            steps {
                script {
                    def tag = env.GIT_COMMIT?.take(7) ?: 'latest'
                    withCredentials([file(credentialsId: 'kubeconfig-staging', variable: 'KUBECONFIG')]) {
                        sh """
                            helm upgrade --install ${HELM_RELEASE} ./helm/rust-conductor \
                              --namespace conductor-staging \
                              --create-namespace \
                              --set backend.image.tag=${tag} \
                              --set frontend.image.tag=${tag} \
                              --values helm/rust-conductor/values.yaml \
                              --wait --timeout 5m
                        """
                    }
                }
            }
        }

        // ── Deploy Production ───────────────────────────────────────
        stage('Deploy Production') {
            when {
                allOf {
                    branch 'main'
                    expression { return params.DEPLOY_ENV == 'production' }
                }
            }
            input {
                message 'Deploy to production?'
                ok 'Deploy'
                submitter 'admin,devops'
            }
            steps {
                script {
                    def tag = env.GIT_COMMIT?.take(7) ?: 'latest'
                    withCredentials([file(credentialsId: 'kubeconfig-production', variable: 'KUBECONFIG')]) {
                        sh """
                            helm upgrade --install ${HELM_RELEASE} ./helm/rust-conductor \
                              --namespace conductor-production \
                              --create-namespace \
                              --set backend.image.tag=${tag} \
                              --set frontend.image.tag=${tag} \
                              --values helm/rust-conductor/values-production.yaml \
                              --wait --timeout 10m
                        """
                    }
                }
            }
        }
    }

    post {
        success {
            echo "Pipeline completed successfully for ${env.BRANCH_NAME} (${env.GIT_COMMIT?.take(7)})"
        }
        failure {
            echo "Pipeline FAILED for ${env.BRANCH_NAME} (${env.GIT_COMMIT?.take(7)})"
        }
        cleanup {
            // Remove local images to save disk
            script {
                def tag = env.GIT_COMMIT?.take(7) ?: 'latest'
                sh "docker rmi ${BACKEND_IMAGE}:${tag} || true"
                sh "docker rmi ${FRONTEND_IMAGE}:${tag} || true"
            }
        }
    }
}
