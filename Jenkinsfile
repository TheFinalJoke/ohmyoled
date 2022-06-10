pipeline {
  agent {
    docker {
      image 'python:3.9-bullseye'
    }

  }
  stages {
    stage('Build Python Package') {
      steps {
        sh '''cd src/python

pip install -e .

twine upload dist/* --non-interactive

'''
      }
    }

  }
}