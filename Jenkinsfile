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

sudo /usr/local/bin/python -m pip install --upgrade pip

sudo pip install -e .

cd

sudo twine upload src/python/dist/* --non-interactive

'''
      }
    }

  }
}